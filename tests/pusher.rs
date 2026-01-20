/*
 *   Matrix Hedwig
 *   Copyright (C) 2019, 2020, 2021, 2022& Famedly GmbH
 *
 *   This program is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU Affero General Public License as
 *   published by the Free Software Foundation, either version 3 of the
 *   License, or (at your option) any later version.
 *
 *   This program is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *   GNU Affero General Public License for more details.
 *
 *   You should have received a copy of the GNU Affero General Public License
 *   along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
//! Tests for the pusher.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::{path::PathBuf, sync::Arc};

use a2::{
	request::payload::{Payload, PayloadLike},
	PushType,
};
use async_trait::async_trait;
use axum::{
	body::Body,
	http::{
		header::{CONTENT_LENGTH, CONTENT_TYPE},
		StatusCode,
	},
	Router,
};
use color_eyre::Report;
use firebae_cm::{FcmError, MessageBody};
use matrix_hedwig::{
	api::{create_router, AppState},
	apns::APNSSender,
	error::{ErrCode, HedwigError},
	fcm::FcmSender,
	models::{ApnsHeaders, ApnsPayload, Metrics, NotificationMethod},
	settings::{self, DeserializablePushType, Settings},
};
use opentelemetry::{metrics::MeterProvider, KeyValue};
use opentelemetry_sdk::{metrics::SdkMeterProvider, Resource};
use regex::Regex;
use rust_telemetry::config::OtelConfig;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tower::Service;

#[derive(Debug)]
struct FakeFcmSender(mpsc::Sender<MessageBody>);
#[async_trait]
impl FcmSender for FakeFcmSender {
	async fn send(&self, message: MessageBody) -> Result<String, HedwigError> {
		let should_fail = format!("{message:?}").contains("fcm_fail_pls");

		self.0.send(message).await.unwrap();
		if should_fail {
			Err(firebae_cm::Error::FcmError(FcmError {
				code: 0,
				status: "Bad Request".to_owned(),
				message: "blubb".to_owned(),
			})
			.into())
		} else {
			Ok("owo".to_owned())
		}
	}
}

#[derive(Debug)]
struct FakeAPNSSender {
	tx: mpsc::Sender<Payload>,
}
#[async_trait]
impl APNSSender for FakeAPNSSender {
	async fn send(&self, payload: Payload) -> Result<(), HedwigError> {
		let should_fail = payload.device_token.contains("apns_fail_pls");

		self.tx.send(payload).await.unwrap();
		if should_fail {
			Err(HedwigError { error: "Bad Request".to_owned(), errcode: ErrCode::APNSFailed })
		} else {
			Ok(())
		}
	}
}

fn setup_server(
	fcm_sender: Box<dyn FcmSender + Send + Sync>,
	apns_sender: Option<Box<dyn APNSSender + Send + Sync>>,
) -> Result<Router, Report> {
	let settings = {
		let log = settings::Log { level: "DEBUG".to_owned() };

		let server = settings::Server { port: 4567, bind_address: [0, 0, 0, 0].into() };

		let hedwig = settings::Hedwig {
			app_id: "com.famedly.🦊".to_owned(),
			push_max_retries: 4,
			notification_title: "🦊 <count> 🦊".to_owned(),
			notification_body: "read the notification pls :c".to_owned(),
			notification_sound: "default".to_owned(),
			notification_android: settings::FcmNotificationAndroid {
				icon: "notifications_icon".to_owned(),
				tag: "org.matrix.default_notification".to_owned(),
				channel_id: "org.matrix.app.message".to_owned(),
				color: None,
				body_loc_key: None,
				body_loc_args: None,
				title_loc_key: None,
				title_loc_args: None,
				ticker: None,
				sticky: None,
				event_time: None,
				local_only: None,
				default_sound: None,
				notification_priority: None,
				default_vibrate_timings: None,
				default_light_settings: None,
				vibrate_timings: None,
				visibility: None,
				light_settings: None,
				image: None,
			},
			notification_click_action: "FLUTTER_NOTIFICATION_CLICK".to_owned(),
			apns_headers: ApnsHeaders {
				apns_push_type: DeserializablePushType(PushType::Background),
				apns_topic: Some("app.bundle.id".to_owned()),
				apns_collapse_id: None,
				apns_expiration: None,
				apns_id: None,
				apns_priority: Some("5".into()),
			},
			apns_payload: ApnsPayload {
				category: None,
				content_available: None,
				mutable_content: 1,
			},
			notification_request_body_size_limit:
				Settings::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT,
			apns_key_file_path: None,
			fcm_credentials_file_path: PathBuf::from(""),
			apns_key_id: "".to_owned(),
			apns_team_id: "".to_owned(),
			apns_sandbox: false,
		};
		Settings { log, server, hedwig, telemetry: OtelConfig::default() }
	};

	let registry = prometheus::Registry::new();
	let exporter = opentelemetry_prometheus::exporter().with_registry(registry.clone()).build()?;

	let provider = SdkMeterProvider::builder()
		.with_resource(
			Resource::builder().with_attribute(KeyValue::new("service.name", "Hedwig")).build(),
		)
		.with_reader(exporter)
		.build();
	let meter = provider.meter("Hedwig");
	let metrics = Metrics::new(&meter);

	opentelemetry::global::set_meter_provider(provider);

	let app_state = AppState::new(fcm_sender, apns_sender, settings, metrics);

	let router = create_router(app_state, Arc::new(registry))?;

	Ok(router)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Platform {
	Android,
	AndroidLegacy,
	IoS,
	Generic,
}

fn get_device(app_id: &str, platform: Platform, notify_via: NotificationMethod) -> Value {
	let (app_id, data) = match platform {
		Platform::Android => (
			app_id.to_owned(),
			json!({
					"format": "event_id_only",
					"data_message": "android"
				}
			),
		),
		Platform::AndroidLegacy => (
			format!("{app_id}.data_message"),
			json!({
					"format": "event_id_only"
				}
			),
		),
		Platform::IoS => (
			app_id.to_owned(),
			json!({
					"format": "event_id_only",
					"data_message": "ios"
				}
			),
		),
		Platform::Generic => (
			app_id.to_owned(),
			json!({
					"format": "event_id_only"
				}
			),
		),
	};

	let mut device = json!({
		"app_id": app_id,
		"data": data,
		"pushkey": format!("{platform:?}"),
		"pushkey_ts": 1_655_896_032_i32,
	});

	if matches!(platform, Platform::IoS) {
		device["notify_via"] = json!(notify_via);
	}

	device
}

fn test_message(clearing: bool, devices: Vec<Value>) -> Value {
	if !clearing {
		json!({
			"notification": {
				"counts": {
					"unread": 1337_i32
				},
				"devices": devices,
				"room_id": "owo",
				"event-id": "uwu",
				"prio": "high"
			}
		})
	} else {
		json!({
			"notification": {
				"counts": {
					"unread": 0
				},
				"devices": devices,
				"prio": "high"
			}
		})
	}
}

/// Extracts Axum response into string.
///
/// Fails if Response is invalid, or the contained data is not an UTF8 String.
async fn response_to_string(
	response: axum::response::Response,
) -> Result<String, Box<dyn std::error::Error>> {
	let body_data = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
	let string = std::str::from_utf8(&body_data)?.to_owned();

	Ok(string)
}

async fn run_request(
	service: &mut Router,
	body: Value,
) -> Result<String, Box<dyn std::error::Error>> {
	let body = serde_json::to_string(&body)?;

	let resp = service
		.call(
			axum::http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.len())
				.body(Body::from(body))?,
		)
		.await?;

	response_to_string(resp).await
}

async fn check_prom(
	service: &mut Router,
	filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let resp = service.call(axum::http::Request::get("/metrics").body(Body::empty())?).await?;
	let data = response_to_string(resp).await?;

	let re = Regex::new(r"} [0-9]\.[0-9]+")?;
	let data = re.replace_all(&data, "} FLOAT");

	// any version of the telemetry sdk is fine. we do this to avoid test failure on
	// simple version bumping
	let processed_data = Regex::new(r#"telemetry_sdk_version="[^"]*""#)
		.unwrap()
		.replace_all(&data, r#"telemetry_sdk_version="any""#);
	assert_eq!(processed_data.trim_end(), std::fs::read_to_string(filename)?.trim_end());
	Ok(())
}

#[tokio::test]
async fn fcm_failure() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, mut _fcm_rx) = mpsc::channel(1337);
	let (apns_tx, mut _apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	let msg = json!({
		"notification": {
			"counts": {
				"unread": 1337_i32
			},
			"devices": [get_device("com.famedly.🦊", Platform::Android, NotificationMethod::Fcm)],
			"room_id": "fcm_fail_pls",
			"event-id": "uwu",
			"prio": "high"
		}
	});
	assert_eq!("{\"rejected\":[\"Android\"]}", run_request(&mut service, msg).await?);

	Ok(())
}

#[tokio::test]
async fn apns_through_fcm_failure() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, mut _fcm_rx) = mpsc::channel(1337);
	let (apns_tx, mut _apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	let msg = json!({
		"notification": {
			"counts": {
				"unread": 1337_i32
			},
			"devices": [get_device("apns_fail_pls", Platform::IoS, NotificationMethod::Fcm)],
			"room_id": "whatever",
			"event-id": "uwu",
			"prio": "high"
		}
	});
	assert_eq!("{\"rejected\":[\"IoS\"]}", run_request(&mut service, msg).await?);

	Ok(())
}

#[tokio::test]
async fn direct_apns_failure() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, mut _fcm_rx) = mpsc::channel(1337);
	let (apns_tx, mut _apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	let msg = json!({
		"notification": {
			"counts": {
				"unread": 1337_i32
			},
			"devices": [get_device("apns_fail_pls", Platform::IoS, NotificationMethod::Apns)],
			"room_id": "whatever",
			"event-id": "uwu",
			"prio": "high"
		}
	});
	assert_eq!("{\"rejected\":[\"IoS\"]}", run_request(&mut service, msg).await?);

	Ok(())
}

#[tokio::test]
async fn bad_json() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, mut _fcm_rx) = mpsc::channel(1337);
	let (apns_tx, mut _apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	let body = "I hate json";

	let resp = service
		.call(
			axum::http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.len())
				.body(Body::from(body))?,
		)
		.await?;
	let data = response_to_string(resp).await?;

	assert_eq!(
		&data,
		"{\"error\":\"Failed to parse the request body as JSON: expected value at line 1 column 1\",\"errcode\":\"BAD_JSON\"}"
	);
	Ok(())
}

#[tokio::test]
async fn push_body_limit() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, _fcm_rx) = mpsc::channel(1337);
	let (apns_tx, _apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	let body_limit: usize =
		Settings::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT.try_into().unwrap();
	let too_long_content = format!("com.famedly.{}", "🐉".repeat(body_limit));

	let device = vec![get_device(&too_long_content, Platform::Android, NotificationMethod::Fcm)];
	let too_long_message = test_message(false, device);
	let body = serde_json::to_string(&too_long_message)?;

	let resp = service
		.call(
			axum::http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.len())
				.body(Body::from(body))?,
		)
		.await?;

	assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

	let data = response_to_string(resp).await?;
	assert_eq!(&data, "{\"error\":\"Failed to buffer the request body: length limit exceeded\",\"errcode\":\"BAD_JSON\"}");

	Ok(())
}

#[tokio::test]
async fn normal_operation() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, mut fcm_rx) = mpsc::channel(1337);
	let (apns_tx, mut apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	for (clearing, platform, filename, notify_via) in [
		(true, Platform::Android, "tests/message_android_clearing.json", NotificationMethod::Fcm),
		(false, Platform::Android, "tests/message_android.json", NotificationMethod::Fcm),
		(
			true,
			Platform::AndroidLegacy,
			"tests/message_android_legacy_clearing.json",
			NotificationMethod::Fcm,
		),
		(
			false,
			Platform::AndroidLegacy,
			"tests/message_android_legacy.json",
			NotificationMethod::Fcm,
		),
		(true, Platform::Generic, "tests/message_generic_clearing.json", NotificationMethod::Fcm),
		(false, Platform::Generic, "tests/message_generic.json", NotificationMethod::Fcm),
		(true, Platform::IoS, "tests/message_ios_fcm_clearing.json", NotificationMethod::Fcm),
		(false, Platform::IoS, "tests/message_ios_fcm.json", NotificationMethod::Fcm),
		(
			true,
			Platform::IoS,
			"tests/message_ios_direct_apns_clearing.json",
			NotificationMethod::Apns,
		),
		(false, Platform::IoS, "tests/message_ios_direct_apns.json", NotificationMethod::Apns),
	] {
		let resp = run_request(
			&mut service,
			test_message(
				clearing,
				vec![get_device("com.famedly.🦊", platform.clone(), notify_via.clone())],
			),
		)
		.await?;

		let posted_message = match notify_via {
			NotificationMethod::Apns => apns_rx.recv().await.unwrap().to_json_string()?,
			_ => serde_json::to_string(&fcm_rx.recv().await.unwrap())?,
		};

		assert_eq!(&resp, "{\"rejected\":[]}");
		assert_eq!(posted_message, std::fs::read_to_string(filename)?.trim_end());
	}

	check_prom(&mut service, "tests/normal_operation_prometheus.txt").await?;

	Ok(())
}

#[tokio::test]
async fn many_requests() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, mut fcm_rx) = mpsc::channel(1337);
	let (apns_tx, mut apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	// with apns
	let dev = vec![get_device("com.famedly.🦊", Platform::IoS, NotificationMethod::Apns)];

	for _ in 1..100 {
		run_request(&mut service, test_message(false, dev.clone())).await?;
		apns_rx.recv().await.unwrap();
	}

	// with fcm
	let dev = vec![get_device("com.famedly.🦊", Platform::IoS, NotificationMethod::Fcm)];

	for _ in 1..100 {
		run_request(&mut service, test_message(false, dev.clone())).await?;
		fcm_rx.recv().await.unwrap();
	}

	Ok(())
}

#[tokio::test]
async fn many_devices() -> Result<(), Box<dyn std::error::Error>> {
	let (fcm_tx, mut fcm_rx) = mpsc::channel(1337);
	let (apns_tx, _apns_rx) = mpsc::channel(1337);
	let mut service = setup_server(
		Box::new(FakeFcmSender(fcm_tx)),
		Some(Box::new(FakeAPNSSender { tx: apns_tx })),
	)?;

	// Success
	for clearing in [true, false] {
		let devices =
			[Platform::Android, Platform::AndroidLegacy, Platform::IoS, Platform::Generic]
				.map(|platform| get_device("com.famedly.🦊", platform, NotificationMethod::Fcm))
				.to_vec();

		let resp = run_request(&mut service, test_message(clearing, devices)).await?;
		assert_eq!(&resp, "{\"rejected\":[]}");

		for (_, filename) in [
			(Platform::Android, "tests/message_android"),
			(Platform::AndroidLegacy, "tests/message_android_legacy"),
			(Platform::IoS, "tests/message_ios_fcm"),
			(Platform::Generic, "tests/message_generic"),
		]
		.map(|(p, n)| {
			(p, if clearing { format!("{n}_clearing.json") } else { format!("{n}.json") })
		}) {
			let posted_message = serde_json::to_string(&fcm_rx.recv().await.unwrap())?;
			assert_eq!(posted_message, std::fs::read_to_string(filename)?.trim_end());
		}
	}

	// Partial failure
	let devices = vec![
		get_device("com.famedly.🐾", Platform::AndroidLegacy, NotificationMethod::Fcm),
		get_device("com.famedly.🦊", Platform::Android, NotificationMethod::Fcm),
		get_device("com.famedly.🐾", Platform::Generic, NotificationMethod::Fcm),
		get_device("com.famedly.🦊", Platform::IoS, NotificationMethod::Fcm),
	];
	let resp = run_request(&mut service, test_message(true, devices)).await?;
	assert_eq!(&resp, "{\"rejected\":[\"AndroidLegacy\",\"Generic\"]}");
	check_prom(&mut service, "tests/many_devices_prometheus.txt").await?;

	Ok(())
}

#[derive(Debug)]
struct PanickingFcmSender;
#[async_trait]
impl FcmSender for PanickingFcmSender {
	async fn send(&self, _message: MessageBody) -> Result<String, HedwigError> {
		panic!("Run for your lives!");
	}
}

#[derive(Debug)]
struct PanickingAPNSSender {}
#[async_trait]
impl APNSSender for PanickingAPNSSender {
	async fn send(&self, _payload: Payload) -> Result<(), HedwigError> {
		panic!("Run for your lives!");
	}
}

#[tokio::test]
async fn panic_handler() -> Result<(), Box<dyn std::error::Error>> {
	let mut service =
		setup_server(Box::new(PanickingFcmSender), Some(Box::new(PanickingAPNSSender {})))?;

	let body = serde_json::to_string(&test_message(
		false,
		vec![get_device("com.famedly.🦊", Platform::IoS, NotificationMethod::Fcm)],
	))?;
	let resp = service
		.call(
			axum::http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.len())
				.body(Body::from(body))?,
		)
		.await?;
	let status = resp.status();
	let response_string = response_to_string(resp).await?;
	assert_eq!(
		status,
		StatusCode::INTERNAL_SERVER_ERROR,
		"Got incorrect error code: {response_string}"
	);

	Ok(())
}
