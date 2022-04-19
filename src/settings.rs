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

use std::collections::HashMap;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

/// Apns Endpoint settings
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ApnsEndpoint {
	/// The Apns endpoint is a production endpoint
	Production,
	/// The Apns endpoint is a sandbox endpoint
	Sandbox,
}

impl From<ApnsEndpoint> for a2::Endpoint {
	fn from(val: ApnsEndpoint) -> Self {
		match val {
			ApnsEndpoint::Production => Self::Production,
			ApnsEndpoint::Sandbox => Self::Sandbox,
		}
	}
}

/// Pusher config for an FCM pusher, where the pusher receives the cleartext
/// notif
#[derive(Debug, Deserialize, Clone)]
pub struct FcmPusher {
	/// The FCM admin key
	pub admin_key: String,
	/// The android click action for the resulting notification
	pub click_action: Option<String>,
	/// Notification icon
	pub icon: Option<String>,
	/// Notification tag
	pub tag: Option<String>,
}

/// Pusher config for an FCM data pusher, where the client can process the
/// notification themself
#[derive(Debug, Deserialize, Clone)]
pub struct FcmDataPusher {
	/// The FCM admin key
	pub admin_key: String,
}

/// Pusher config for an APNS pusher
#[derive(Debug, Deserialize, Clone)]
pub struct ApnsPusher {
	/// The key file for APNS
	pub key_file: String,
	/// The key id of the specified key file
	pub key_id: String,
	/// The team id for APNS
	pub team_id: String,
	/// Optionally, the topic needed for APNS
	pub topic: Option<String>,
	/// Which APNS endpoint to use, one of production or sandbox
	pub endpoint: ApnsEndpoint,
}

/// Pusher config
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Pusher {
	/// Pusher config for an FCM pusher, where the pusher receives the cleartext
	/// notif
	Fcm(FcmPusher),
	/// Pusher config for an FCM data pusher, where the client can process the
	/// notification themself
	FcmData(FcmDataPusher),
	/// Pusher config for an APNS pusher
	Apns(ApnsPusher),
}

/// Notification configuration
#[derive(Deserialize, Debug, Clone)]
pub struct Notification {
	/// The text to display in a notification (replaces <count> tag with a
	/// notification count
	pub title: String,
	/// The text to display as a notification body
	pub body: String,
	/// What sound should be played as a part of notification
	pub sound: Option<String>,
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
	/// Notification settings
	pub notification: Notification,
	/// Pushers settings
	pub pushers: HashMap<String, Pusher>,
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
