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

use config::{Config, ConfigError, Environment, File};
use serde::{de::Error, Deserialize, Deserializer};
use tracing_appender::rolling::Rotation;

/// Hedwig configuration
#[derive(Debug, Deserialize)]
pub struct Hedwig {
	/// Application ID
	pub app_id: String,
	/// Maximum amount of jitter that can be introduced to notifications in
	/// seconds (msc3359), 0 to disable
	pub max_jitter_delay: f64,
	/// Maximum amount of attempts hedwig should make
	pub fcm_push_max_retries: i64,
	/// FCM Administration key
	pub fcm_service_account_token_path: String,
	/// The text to display in a notification (replaces <count> tag with a
	/// notification count
	pub fcm_notification_title: String,
	/// The text to display as a notification body
	pub fcm_notification_body: String,
	/// What sound should be played as a part of notification
	pub fcm_notification_sound: String,
	/// Notification icon
	pub fcm_notification_icon: String,
	/// Notification tag
	pub fcm_notification_tag: String,
	/// ID of the android channel
	pub fcm_notification_android_channel_id: String,
	/// Action to trigger on the notification click
	pub fcm_notification_click_action: String,
}

/// Push gateway server configuration
#[derive(Debug, Deserialize)]
pub struct Server {
	/// Push gateway port
	pub port: u16,
	/// IP address the server is listening on
	pub bind_address: IpAddr,
}

/// Log file output settings
#[derive(Debug, Deserialize, Clone)]
pub struct LogFileOuput {
	/// Log directory
	pub directory: String,
	/// Log prefix
	pub prefix: String,
	/// Log file rolling frequency: (MINUTELY, HOURLY, DAILY, NEVER)
	#[serde(deserialize_with = "rolling_from_str")]
	pub rolling_frequency: Rotation,
}

/// Log settings
#[derive(Debug, Deserialize)]
pub struct Log {
	/// Log level (DEBUG, INFO, ERROR etc.)
	pub level: String,
	/// File output options
	pub file_output: Option<LogFileOuput>,
}

/// Converts a string into a Rolling frequency
fn rolling_from_str<'de, D>(deserializer: D) -> Result<Rotation, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;

	match s.as_str() {
		"MINUTELY" => Ok(Rotation::MINUTELY),
		"HOURLY" => Ok(Rotation::HOURLY),
		"DAILY" => Ok(Rotation::DAILY),
		"NEVER" => Ok(Rotation::NEVER),
		_ => Err(D::Error::custom(
			"Log rolling frequency must be one of MINUTELY, HOURLY, DAILY, NEVER",
		)),
	}
}

/// Main settings struct
#[derive(Debug, Deserialize)]
pub struct Settings {
	/// Log settings
	pub log: Log,
	/// Server settings
	pub server: Server,
	/// Hedwig settings
	pub hedwig: Hedwig,
}

impl Settings {
	/// Load settings from file
	pub fn load(filename: &str) -> Result<Self, ConfigError> {
		Config::builder()
			.add_source(File::with_name(filename))
			.add_source(Environment::with_prefix("push_gw").separator("_"))
			.set_default("log.level", "INFO")?
			.build()?
			.try_deserialize()
	}
}
