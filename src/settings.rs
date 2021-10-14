//! Hedwig settings

/*
 *   Matrix Hedwig
 *   Copyright (C) 2019, 2020, 2021 Famedly GmbH
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

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

/// Hedwig configuration
#[derive(Deserialize, Debug, Clone)]
pub struct Hedwig {
	/// Application ID
	pub app_id: String,
	/// FCM Administration key
	pub fcm_admin_key: String,
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
#[derive(Debug, Deserialize, Clone)]
pub struct Server {
	/// Push gateway port
	pub port: u16,
	/// IP address the server is listening on
	pub bind_address: String,
}

/// Log settings
#[derive(Debug, Deserialize, Clone)]
pub struct Log {
	/// Log level (DEBUG, INFO, ERROR etc.)
	pub level: String,
}

/// Main settings struct
#[derive(Debug, Deserialize, Clone)]
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
	pub fn load() -> Result<Self, ConfigError> {
		let mut conf = Config::new();
		conf.merge(File::with_name("config.yaml"))?;
		conf.merge(Environment::with_prefix("push_gw").separator("_"))?;
		conf.set_default("log.level", "INFO")?;
		conf.try_into()
	}
}
