/*
 *   Matrix Hedwig
 *   Copyright (C) 2019, 2020, 2021, 2022 Famedly GmbH
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

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
	body::{Body, HttpBody},
	routing::{get, post},
	Extension, Router,
};
use firebae_cm::MessageBody;
use futures::future::poll_fn;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use matrix_hedwig::{
	api::matrix_push,
	error::HedwigError,
	fcm::FcmSender,
	jitter::Jitter,
	models,
	settings::{self, Settings},
};
use regex::Regex;
use serde_json::json;
use tokio::sync::{mpsc, Mutex, RwLock};
use tower::Service;

struct FakeSender(mpsc::Sender<MessageBody>);
#[async_trait]
impl FcmSender for FakeSender {
	async fn send(&self, message: MessageBody) -> Result<String, HedwigError> {
		self.0.send(message).await.unwrap();
		Ok("owo".to_owned())
	}
}

async fn setup_server(fcm_sender: Box<dyn FcmSender + Send + Sync>) -> Router<Body> {
	let settings = {
		let log = settings::Log { level: "DEBUG".to_owned() };

		let server = settings::Server { port: 4567, bind_address: [0, 0, 0, 0].into() };

		let hedwig = settings::Hedwig {
			app_id: "com.famedly.🦊".to_owned(),
			max_jitter_delay: 2.0,
			fcm_push_max_retries: 4,
			fcm_service_account_token_path: "placeholder".to_owned(),
			fcm_notification_title: "🦊 <count> 🦊".to_owned(),
			fcm_notification_body: "read the notification pls :c".to_owned(),
			fcm_notification_sound: "default".to_owned(),
			fcm_notification_icon: "notifications_icon".to_owned(),
			fcm_notification_tag: "org.matrix.default_notification".to_owned(),
			fcm_notification_android_channel_id: "org.matrix.app.message".to_owned(),
			fcm_notification_click_action: "FLUTTER_NOTIFICATION_CLICK".to_owned(),
		};
		Settings { log, server, hedwig }
	};

	let jitter = Jitter::new(Duration::from_secs_f64(0.0));
	let metrics_middleware =
		axum_opentelemetry_middleware::RecorderMiddlewareBuilder::new("Hedwig");
	let counters = models::Metrics::new(&metrics_middleware.meter);

	let mut service = Router::new()
		.route("/metrics", get(axum_opentelemetry_middleware::metrics_endpoint))
		.route("/_matrix/push/v1/notify", post(matrix_push))
		.layer(Extension(Arc::new(RwLock::new(jitter))))
		.layer(Extension(Arc::new(Mutex::new(fcm_sender))))
		.layer(Extension(Arc::new(settings)))
		.layer(Extension(Arc::new(counters)))
		.layer(metrics_middleware.build());

	poll_fn(|cx| service.poll_ready(cx)).await.unwrap();

	service
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
		"pushkey": format!("{:?}", platform),
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

async fn run_request(
	service: &mut Router<Body>,
	body: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
	let body = serde_json::to_string(&body)?;

	let mut resp = service
		.call(
			http::Request::post("/_matrix/push/v1/notify")
				.header(CONTENT_TYPE, "application/json")
				.header(CONTENT_LENGTH, body.as_bytes().len())
				.body(axum::body::Body::from(body))?,
		)
		.await?;
	let data = resp.body_mut().data().await.unwrap()?;

	Ok(std::str::from_utf8(&data)?.to_owned())
}

async fn check_prom(
	service: &mut Router<Body>,
	filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut resp =
		service.call(http::Request::get("/metrics").body(axum::body::Body::empty())?).await?;
	let data = resp.body_mut().data().await.unwrap()?;
	let data = std::str::from_utf8(&data)?.to_owned();
	let re = Regex::new(r"[0-9]\.[0-9]+")?;
	let data = re.replace_all(&data, "FLOAT");

	assert_eq!(data, std::fs::read_to_string(filename)?);
	Ok(())
}

#[tokio::test]
async fn normal_operation() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, mut rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await;

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
async fn many_devices() -> Result<(), Box<dyn std::error::Error>> {
	let (tx, mut rx) = mpsc::channel(1337);
	let mut service = setup_server(Box::new(FakeSender(tx))).await;

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
