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

use std::{fmt, net::IpAddr, path::PathBuf};

use a2::PushType;
use config::{Config, ConfigError, Environment, File};
use firebae_cm::{LightSettings, NotificationPriority, Visibility};
use rust_telemetry::config::OtelConfig;
use serde::{de, Deserialize, Deserializer};

use crate::models::{ApnsHeaders, ApnsPayload};

/// FCM notification Android-specific configuration
/// https://firebase.google.com/docs/reference/fcm/rest/v1/projects.messages#androidnotification
#[derive(Debug, Deserialize)]
pub struct FcmNotificationAndroid {
	/// Notification icon
	pub icon: String,
	/// Notification tag
	pub tag: String,
	/// ID of the android channel
	pub channel_id: String,
	/// The notification's icon color, expressed in #rrggbb format
	pub color: Option<String>,
	/// The key to the body string in the app's string resources to use to
	/// localize the body text to the user's current localization
	pub body_loc_key: Option<String>,
	/// Variable string values to be used in place of the format specifiers in
	/// body_loc_key to use to localize the body text to the user's current
	/// localization
	pub body_loc_args: Option<Vec<String>>,
	/// The key to the title string in the app's string resources to use to
	/// localize the title text to the user's current localization
	pub title_loc_key: Option<String>,
	/// Variable string values to be used in place of the format specifiers in
	/// title_loc_key to use to localize the title text to the user's current
	/// localization
	pub title_loc_args: Option<Vec<String>>,
	/// Sets the "ticker" text, which is sent to accessibility services. Prior
	/// to API level 21 (Lollipop), sets the text that is displayed in the
	/// status bar when the notification first arrives.
	pub ticker: Option<String>,
	/// When set to false or unset, the notification is automatically dismissed
	/// when the user clicks it in the panel. When set to true, the notification
	/// persists even when the user clicks it.
	pub sticky: Option<bool>,
	/// Set the time that the event in the notification occurred. Notifications
	/// in the panel are sorted by this time.
	pub event_time: Option<time::OffsetDateTime>,
	/// Set whether or not this notification is relevant only to the current
	/// device. Some notifications can be bridged to other devices for remote
	/// display, such as a Wear OS watch.
	pub local_only: Option<bool>,
	/// If set to true, use the Android framework's default sound for the
	/// notification.
	pub default_sound: Option<bool>,
	/// Set the relative priority for this notification
	pub notification_priority: Option<NotificationPriority>,
	/// If set to true, use the Android framework's default vibrate pattern for
	/// the notification.
	pub default_vibrate_timings: Option<bool>,
	/// If set to true, use the Android framework's default LED light settings
	/// for the notification.
	pub default_light_settings: Option<bool>,
	/// Set the vibration pattern to use.
	pub vibrate_timings: Option<Vec<String>>,
	/// Set the Notification.visibility of the notification.
	pub visibility: Option<Visibility>,
	/// Sets the number of items this notification represents.
	pub light_settings: Option<LightSettings>,
	/// Contains the URL of an image that is going to be displayed in a
	/// notification.
	pub image: Option<String>,
}

/// Hedwig configuration
#[derive(Debug, Deserialize)]
pub struct Hedwig {
	/// Application ID
	pub app_id: String,
	/// Maximum amount of attempts hedwig should make
	pub push_max_retries: i64,
	/// The text to display in a notification (replaces <count> tag with a
	/// notification count
	pub notification_title: String,
	/// The text to display as a notification body
	pub notification_body: String,
	/// What sound should be played as a part of notification
	pub notification_sound: String,
	/// FCM notification Android-specific configuration
	pub notification_android: FcmNotificationAndroid,
	/// Headers sent to APNS
	pub apns_headers: ApnsHeaders,
	/// Payload sent to APNS
	pub apns_payload: ApnsPayload,

	/// Action to trigger on the notification click
	pub notification_click_action: String,
	/// Path to the APNs key file
	pub apns_key_file_path: Option<PathBuf>,
	/// Path to the FCM credentials file
	pub fcm_credentials_file_path: PathBuf,
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
/// Ideally, the Deserialize trait should be implemented in the a2 crate
/// directly
#[derive(Debug, Clone)]
pub struct DeserializablePushType(pub PushType);

impl fmt::Display for DeserializablePushType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl<'de> Deserialize<'de> for DeserializablePushType {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?.to_lowercase();

		let push_type = match s.as_str() {
			"alert" => PushType::Alert,
			"background" => PushType::Background,
			// "location" => PushType::Location,
			// "voip" => PushType::Voip,
			// "fileprovider" => PushType::FileProvider,
			// "mdm" => PushType::Mdm,
			// "liveactivity" => PushType::LiveActivity,
			// "pushtotalk" => PushType::PushToTalk,
			_ => return Err(de::Error::custom(format!("Unknown PushType: '{s}'",))),
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
