//! Data structure for generic way to send messages to the real APNS instance
//! while allowing to easily mock the behaviour

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

use std::{fmt::Debug, fs::File};

use a2::{
	Client, ClientConfig, DefaultNotificationBuilder, Endpoint, NotificationBuilder,
	NotificationOptions, PushType,
};
use async_trait::async_trait;

use crate::error::{ErrCode, HedwigError};

/// Trait for allowing the use of different senders for APNS messages
/// Mainly this way to make testing possible
#[async_trait]
pub trait APNSSender<'a>: Debug {
	/// Send off a message to APNS
	async fn send(
		&self,
		builder: DefaultNotificationBuilder<'a>,
		device_token: &str,
	) -> Result<(), HedwigError>;
}

/// Default implementation for FcmSender
#[derive(Debug)]
pub struct APNSSenderImpl {
	/// Client for sending the message
	client: Client,
	/// Usually the bundle ID of the app
	topic: String,
	/// APNS push-type header
	push_type: PushType,
}

impl APNSSenderImpl {
	/// Create new APNS sender from the path to an APNS private key (.p8 file)
	pub fn new(topic: String, push_type: PushType) -> Result<Self, HedwigError> {
		let key_file = String::new();
		let team_id = String::new();
		let key_id = String::new();
		let sandbox = false;

		let mut private_key = File::open(key_file).map_err(|e| HedwigError {
			error: e.to_string(),
			errcode: ErrCode::APNSPrivateKeyNotFound,
		})?;

		// Which service to call, test or production?
		let endpoint = if sandbox { Endpoint::Sandbox } else { Endpoint::Production };

		let client_config = ClientConfig::new(endpoint);

		// Connecting to APNs
		let client = Client::token(&mut private_key, key_id, team_id, client_config)
			.map_err(|e| HedwigError { error: e.to_string(), errcode: ErrCode::APNSAuthFailed })?;

		Ok(Self { client, topic, push_type })
	}
}

#[async_trait]
impl<'a> APNSSender<'a> for APNSSenderImpl {
	async fn send(
		&self,
		builder: DefaultNotificationBuilder<'a>,
		device_token: &str,
	) -> Result<(), HedwigError> {
		let options = NotificationOptions {
			apns_topic: Some(&self.topic),
			apns_push_type: Some(self.push_type),
			..Default::default()
		};

		let payload = builder.build(device_token.as_ref(), options);
		let response = self
			.client
			.send(payload)
			.await
			.map_err(|e| HedwigError { errcode: ErrCode::APNSFailed, error: e.to_string() })?;

		if let Some(error) = response.error {
			return Err(HedwigError {
				errcode: ErrCode::APNSFailed,
				error: format!("Failed sending notification to APNS: {}", error.reason),
			});
		}

		Ok(())
	}
}
