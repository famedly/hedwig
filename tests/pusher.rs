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

#![allow(clippy::unwrap_used)]

use async_trait::async_trait;
use axum::{body::Body, Router};
use color_eyre::Report;
use firebae_cm::MessageBody;
use futures::future::poll_fn;
use http::{
	header::{CONTENT_LENGTH, CONTENT_TYPE},
	StatusCode,
};
use matrix_hedwig::{
	api::{create_router, AppState},
	error::HedwigError,
	fcm::FcmSender,
	models,
	settings::{self, Settings},
};
use regex::Regex;
use serde_json::json;
use tokio::sync::mpsc;
use tower::Service;

#[derive(Debug)]
struct FakeSender(mpsc::Sender<MessageBody>);
#[async_trait]
impl FcmSender for FakeSender {
	async fn send(&self, message: MessageBody) -> Result<String, HedwigError> {
		let should_fail = format!("{message:?}").contains("fcm_fail_pls");

		self.0.send(message).await.unwrap();
		if should_fail {
			Err(firebae_cm::Error::Other("blubb".to_owned(), 12).into())
		} else {
			Ok("owo".to_owned())
		}
	}
}

async fn setup_server(fcm_sender: Box<dyn FcmSender + Send + Sync>) -> Result<Router, Report> {
	let settings = {
		let log = settings::Log { file_output: None, level: "DEBUG".to_owned() };

		let server = settings::Server { port: 4567, bind_address: [0, 0, 0, 0].into() };

		let hedwig = settings::Hedwig {
			app_id: "com.famedly.🦊".to_owned(),
			fcm_push_max_retries: 4,
			fcm_service_account_token_path: "placeholder".to_owned(),
			fcm_notification_title: "🦊 <count> 🦊".to_owned(),
			fcm_notification_body: "read the notification pls :c".to_owned(),
			fcm_notification_sound: "default".to_owned(),
			fcm_notification_icon: "notifications_icon".to_owned(),
			fcm_notification_tag: "org.matrix.default_notification".to_owned(),
			fcm_notification_android_channel_id: "org.matrix.app.message".to_owned(),
			fcm_notification_click_action: "FLUTTER_NOTIFICATION_CLICK".to_owned(),
			notification_request_body_size_limit:
				Settings::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT,
		};
		Settings { log, server, hedwig }
	};

	let metrics_middleware =
		axum_opentelemetry_middleware::RecorderMiddlewareBuilder::new("Hedwig");
	let counters = models::Metrics::new(&metrics_middleware.meter);

	let app_state = AppState::new(fcm_sender, settings, counters);

	let mut router = create_router(app_state, metrics_middleware.build())?;

	poll_fn(|cx| router.poll_ready(cx)).await.unwrap();

	Ok(router)
}

#[derive(Debug)]
enum Platform {
	Android,
	AndroidLegacy,
	IoS,
	Generic,
}

fn get_device(app_id: &str, platform: Platform) -> serde_json::Value {
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

	json!({
		"app_id": app_id,
		"data": data,
		"pushkey": format!("{platform:?}"),
		"pushkey_ts": 1_655_896_032_i32
	})
}

fn test_message(clearing: bool, devices: Vec<serde_json::Value>) -> serde_json::Value {
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
	let body_data = hyper::body::to_bytes(response.into_body()).await?;
	let string = std::str::from_utf8(&body_data)?.to_owned();

	Ok(string)
}

async fn run_request(
	service: &mut Router<(), Body>,
	body: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
	let body = serde_json::to_string(&body)?;

	let resp = service
		.call(
			http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.as_bytes().len())
				.body(Body::from(body))?,
		)
		.await?;

	response_to_string(resp).await
}

async fn check_prom(
	service: &mut Router<(), Body>,
	filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let resp =
		service.call(http::Request::get("/metrics").body(Body::empty())?).await?;
	let data = response_to_string(resp).await?;

	let re = Regex::new(r"} [0-9]\.[0-9]+")?;
	let data = re.replace_all(&data, "} FLOAT");

	assert_eq!(data, std::fs::read_to_string(filename)?);
	Ok(())
}

#[tokio::test]
async fn fcm_failure() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, mut _rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await?;

	let msg = json!({
		"notification": {
			"counts": {
				"unread": 1337_i32
			},
			"devices": [get_device("com.famedly.🦊", Platform::IoS)],
			"room_id": "fcm_fail_pls",
			"event-id": "uwu",
			"prio": "high"
		}
	});
	assert_eq!("{\"rejected\":[\"IoS\"]}", run_request(&mut service, msg).await?);

	Ok(())
}

#[tokio::test]
async fn bad_json() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, mut _rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await?;

	let body = "I hate json";

	let resp = service
		.call(
			http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.as_bytes().len())
				.body(Body::from(body))?,
		)
		.await?;
	let data = response_to_string(resp).await?;

	assert_eq!(
		&data,
		"{\"error\":\"Failed to parse the request body as JSON\",\"errcode\":\"BAD_JSON\"}"
	);
	Ok(())
}

#[tokio::test]
async fn push_body_limit() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, _rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await?;

	let body_limit: usize =
		Settings::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT.try_into().unwrap();
	let too_long_content = format!("com.famedly.{}", "🐉".repeat(body_limit));

	let device = vec![get_device(&too_long_content, Platform::Android)];
	let too_long_message = test_message(false, device);
	let body = serde_json::to_string(&too_long_message)?;

	let resp = service
		.call(
			http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.as_bytes().len())
				.body(Body::from(body))?,
		)
		.await?;

	assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

	let data = response_to_string(resp).await?;
	assert_eq!(&data, "{\"error\":\"Failed to buffer the request body\",\"errcode\":\"BAD_JSON\"}");

	Ok(())
}

#[tokio::test]
async fn normal_operation() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, mut rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await?;

	for (clearing, platform, filename) in [
		(true, Platform::Android, "tests/message_android_clearing.json"),
		(false, Platform::Android, "tests/message_android.json"),
		(true, Platform::AndroidLegacy, "tests/message_android_legacy_clearing.json"),
		(false, Platform::AndroidLegacy, "tests/message_android_legacy.json"),
		(true, Platform::Generic, "tests/message_generic_clearing.json"),
		(false, Platform::Generic, "tests/message_generic.json"),
		(true, Platform::IoS, "tests/message_ios_clearing.json"),
		(false, Platform::IoS, "tests/message_ios.json"),
	] {
		let resp = run_request(
			&mut service,
			test_message(clearing, vec![get_device("com.famedly.🦊", platform)]),
		)
		.await?;
		let posted_message = serde_json::to_string(&rx.recv().await.unwrap())?;
		assert_eq!(&resp, "{\"rejected\":[]}");
		assert_eq!(posted_message, std::fs::read_to_string(filename)?);
	}

	check_prom(&mut service, "tests/normal_operation_prometheus.txt").await?;

	Ok(())
}

#[tokio::test]
async fn many_requests() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, mut rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await?;

	let dev = vec![get_device("com.famedly.🦊", Platform::IoS)];

	for _ in 1..100 {
		run_request(&mut service, test_message(false, dev.clone())).await?;
		rx.recv().await.unwrap();
	}

	Ok(())
}

#[tokio::test]
async fn many_devices() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, mut rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await?;

	// Success
	for clearing in [true, false] {
		let devices =
			[Platform::Android, Platform::AndroidLegacy, Platform::IoS, Platform::Generic]
				.map(|platform| get_device("com.famedly.🦊", platform))
				.to_vec();

		let resp = run_request(&mut service, test_message(clearing, devices)).await?;
		assert_eq!(&resp, "{\"rejected\":[]}");

		for filename in [
			"tests/message_android",
			"tests/message_android_legacy",
			"tests/message_ios",
			"tests/message_generic",
		]
		.map(|name| if clearing { format!("{name}_clearing.json") } else { format!("{name}.json") })
		{
			let posted_message = serde_json::to_string(&rx.recv().await.unwrap())?;
			assert_eq!(posted_message, std::fs::read_to_string(filename)?);
		}
	}

	// Partial failure
	let devices = vec![
		get_device("com.famedly.🐾", Platform::AndroidLegacy),
		get_device("com.famedly.🦊", Platform::Android),
		get_device("com.famedly.🐾", Platform::Generic),
		get_device("com.famedly.🦊", Platform::IoS),
	];
	let resp = run_request(&mut service, test_message(true, devices)).await?;
	assert_eq!(&resp, "{\"rejected\":[\"AndroidLegacy\",\"Generic\"]}");
	check_prom(&mut service, "tests/many_devices_prometheus.txt").await?;

	Ok(())
}
