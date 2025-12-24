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
//! Tests for the api server.

use std::path::PathBuf;

use a2::{request::payload::Payload, PushType};
use async_trait::async_trait;
use firebae_cm::MessageBody;
use matrix_hedwig::{
	api::run_server,
	apns::APNSSender,
	error::HedwigError,
	fcm::FcmSender,
	models::{ApnsHeaders, ApnsPayload},
	settings::{self, DeserializablePushType, Settings},
};
use rust_telemetry::config::OtelConfig;
use tokio::time;

#[derive(Debug)]
struct FakeFcmSender;

#[async_trait]
impl FcmSender for FakeFcmSender {
	async fn send(&self, _message: MessageBody) -> Result<String, HedwigError> {
		Ok("test".to_owned())
	}
}

#[derive(Debug)]
struct FakeAPNSSender {}

#[async_trait]
impl APNSSender for FakeAPNSSender {
	async fn send(&self, _payload: Payload) -> Result<(), HedwigError> {
		Ok(())
	}
}

fn create_test_settings(port: u16) -> Settings {
	let log = settings::Log { level: "DEBUG".to_owned() };

	let server = settings::Server { port, bind_address: [127, 0, 0, 1].into() };

	let hedwig = settings::Hedwig {
		app_id: "com.test.app".to_owned(),
		push_max_retries: 3,
		notification_title: "Test".to_owned(),
		notification_body: "Test body".to_owned(),
		notification_sound: "default".to_owned(),
		notification_android: settings::FcmNotificationAndroid {
			icon: "test_icon".to_owned(),
			tag: "test_tag".to_owned(),
			channel_id: "test_channel".to_owned(),
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
		notification_click_action: "TEST_CLICK".to_owned(),
		notification_request_body_size_limit:
			Settings::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT,
		apns_headers: ApnsHeaders {
			apns_push_type: DeserializablePushType(PushType::Background),
			apns_topic: Some("app.bundle.id".to_owned()),
			apns_collapse_id: None,
			apns_expiration: None,
			apns_id: None,
			apns_priority: Some("5".into()),
		},
		apns_payload: ApnsPayload { category: None, content_available: 1, mutable_content: 1 },
		apns_key_file_path: None,
		fcm_credentials_file_path: PathBuf::from(""),
		apns_team_id: "TEAM_ID".to_owned(),
		apns_key_id: "KEY_ID".to_owned(),
		apns_sandbox: false,
	};
	Settings { log, server, hedwig, telemetry: OtelConfig::default() }
}

#[tokio::test]
async fn server_starts_successfully() -> Result<(), Box<dyn std::error::Error>> {
	// Use a high port that's unlikely to be in use
	let settings = create_test_settings(0);
	let fcm_sender: Box<dyn FcmSender + Send + Sync> = Box::new(FakeFcmSender);
	let apns_sender = FakeAPNSSender {};

	let server_handle = tokio::spawn(run_server(settings, fcm_sender, Some(apns_sender)));

	// wait in case an error occurs during startup
	time::sleep(time::Duration::from_secs(1)).await;

	// Server should still be running
	assert!(!server_handle.is_finished());

	server_handle.abort();

	Ok(())
}
