//! Hedwig settings

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

use std::net::IpAddr;

use a2::PushType;
use config::{Config, ConfigError, Environment, File};
use rust_telemetry::config::OtelConfig;
use serde::{de, Deserialize, Deserializer};

/// Hedwig configuration
#[derive(Debug, Deserialize)]
pub struct Hedwig {
	/// Application ID
	pub app_id: String,
	/// Maximum amount of attempts hedwig should make
	pub fcm_push_max_retries: i64,
	/// The text to display in a notification (replaces <count> tag with a
	/// notification count
	pub notification_title: String,
	/// The text to display as a notification body
	pub notification_body: String,
	/// What sound should be played as a part of notification
	pub notification_sound: String,
	/// Notification icon
	pub notification_icon: String,
	/// Notification tag
	pub notification_tag: String,
	/// ID of the android channel
	pub fcm_notification_android_channel_id: String,
	/// Action to trigger on the notification click
	pub notification_click_action: String,
	/// Type of notification for Apple devices
	pub apns_push_type: DeserializablePushType,
	/// Usually the bundle ID of the app
	pub apns_topic: String,
	/// Path to the APNs key file
	pub apns_key_file_path: String,
	/// Path to the FCM credentials file
	pub fcm_credentials_file_path: String,
	/// Team ID of the APNs key
	pub apns_team_id: String,
	/// Key ID of the APNs key
	pub apns_key_id: String,
	/// Whether to use the sandbox environment
	pub apns_sandbox: bool,
	/// Maximum accepted length for NotificationRequests via push
	///
	/// Defaults to [Settings::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT]
	pub notification_request_body_size_limit: u64,
}

/// We need this to implement the Deserialize trait for PushType
#[derive(Debug)]
pub struct DeserializablePushType(pub PushType);

impl<'de> Deserialize<'de> for DeserializablePushType {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?.to_lowercase();

		let push_type = match s.as_str() {
            "alert" => PushType::Alert,
            "background" => PushType::Background,
            "location" => PushType::Location,
            "voip" => PushType::Voip,
            "fileprovider" => PushType::FileProvider,
            "mdm" => PushType::Mdm,
            "liveactivity" => PushType::LiveActivity,
            "pushtotalk" => PushType::PushToTalk,
            _ => return Err(de::Error::custom(format!(
                "Unknown PushType: '{}'. Expected one of: Alert, Background, Location, Voip, FileProvider, Mdm, LiveActivity, PushToTalk",
                s
            ))),
        };

		Ok(DeserializablePushType(push_type))
	}
}

/// Push gateway server configuration
#[derive(Debug, Deserialize)]
pub struct Server {
	/// Push gateway port
	pub port: u16,
	/// IP address the server is listening on
	pub bind_address: IpAddr,
}

/// Log settings
#[derive(Debug, Deserialize)]
pub struct Log {
	/// Log level (DEBUG, INFO, ERROR etc.)
	pub level: String,
}

/// Main settings struct
///
/// The default constants usually get overwritten by the defaults set in
/// "config.sample.yaml"
#[derive(Debug, Deserialize)]
pub struct Settings {
	/// Log settings
	pub log: Log,
	/// Server settings
	pub server: Server,
	/// Hedwig settings
	pub hedwig: Hedwig,
	/// rust-telemetry settings
	pub telemetry: OtelConfig,
}

impl Settings {
	/// Default length limit for the matrix push notifications
	pub const DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT: u64 = 15000;
	/// Hedwig default log level
	pub const DEFAULT_LOG_LEVEL: &'static str = "INFO";
	/// Config filename
	pub const CONFIG_FILENAME: &'static str = "config.yaml";

	/// Load settings from file
	pub fn load(filename: &str) -> Result<Self, ConfigError> {
		Config::builder()
			.add_source(File::with_name(filename).required(false))
			.add_source(Environment::with_prefix("pushgw").prefix_separator("__").separator("__"))
			.set_default("log.level", Self::DEFAULT_LOG_LEVEL)?
			.set_default(
				"hedwig.notification_request_body_size_limit",
				Self::DEFAULT_NOTIFICATION_REQUEST_BODY_SIZE_LIMIT,
			)?
			.build()?
			.try_deserialize()
	}
}
