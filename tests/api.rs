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

use async_trait::async_trait;
use firebae_cm::MessageBody;
use matrix_hedwig::{
	api::run_server,
	error::HedwigError,
	fcm::FcmSender,
	settings::{self, Settings},
};
use rust_telemetry::config::OtelConfig;
use tokio::time;

#[derive(Debug)]
struct FakeSender;
#[async_trait]
impl FcmSender for FakeSender {
	async fn send(&self, _message: MessageBody) -> Result<String, HedwigError> {
		Ok("test".to_owned())
	}
}

fn create_test_settings(port: u16) -> Settings {
	let log = settings::Log { level: "DEBUG".to_owned() };

	let server = settings::Server { port, bind_address: [127, 0, 0, 1].into() };

	let hedwig = settings::Hedwig {
		app_id: "com.test.app".to_owned(),
		fcm_push_max_retries: 3,
		notification_title: "Test".to_owned(),
		notification_body: "Test body".to_owned(),
		notification_sound: "default".to_owned(),
		notification_icon: "test_icon".to_owned(),
		notification_tag: "test_tag".to_owned(),
		fcm_notification_android_channel_id: "test_channel".to_owned(),
		notification_click_action: "TEST_CLICK".to_owned(),
		notification_request_body_size_limit:
			Settings::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT,
		apns_push_type: "background".to_owned(),
	};
	Settings { log, server, hedwig, telemetry: OtelConfig::default() }
}

#[tokio::test]
async fn server_starts_successfully() -> Result<(), Box<dyn std::error::Error>> {
	// Use a high port that's unlikely to be in use
	let settings = create_test_settings(0);
	let fcm_sender: Box<dyn FcmSender + Send + Sync> = Box::new(FakeSender);

	let server_handle = tokio::spawn(run_server(settings, fcm_sender));

	// wait in case an error occurs during startup
	time::sleep(time::Duration::from_secs(1)).await;

	// Server should still be running
	assert!(!server_handle.is_finished());

	server_handle.abort();

	Ok(())
}
