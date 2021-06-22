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

#[derive(Deserialize, Debug, Clone)]
pub struct Hedwig {
	pub app_id: String,
	pub fcm_admin_key: String,
	pub fcm_notification_title: String,
	pub fcm_notification_body: String,
	pub fcm_notification_sound: String,
	pub fcm_notification_icon: String,
	pub fcm_notification_tag: String,
	pub fcm_notification_android_channel_id: String,
	pub fcm_notification_click_action: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
	pub port: u16,
	pub bind_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Log {
	pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
	pub log: Log,
	pub server: Server,
	pub hedwig: Hedwig,
}

impl Settings {
	pub fn load() -> Result<Self, ConfigError> {
		let mut conf = Config::new();
		conf.merge(File::with_name("config.yaml"))?;
		conf.merge(Environment::with_prefix("push_gw").separator("_"))?;
		conf.set_default("log.level", "INFO")?;
		conf.try_into()
	}
}
