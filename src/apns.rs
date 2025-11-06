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

use std::{fmt::Debug, fs::File, path::PathBuf};

use a2::{request::payload::Payload, Client, ClientConfig, Endpoint};
use async_trait::async_trait;

use crate::error::{ErrCode, HedwigError};

/// Trait for allowing the use of different senders for APNS messages
/// This is mainly to make testing possible
#[async_trait]
pub trait APNSSender: Debug {
	/// Send off a message to APNS
	async fn send(&self, payload: Payload) -> Result<(), HedwigError>;
}

/// Default implementation for APNSSender
#[derive(Debug)]
pub struct APNSSenderImpl {
	/// Client for sending the message
	client: Client,
}

impl APNSSenderImpl {
	/// Create new APNS sender from the path to an APNS private key (.p8 file)
	pub fn new(
		key_file_path: PathBuf,
		team_id: String,
		key_id: String,
		sandbox: bool,
	) -> Result<Self, HedwigError> {
		let mut private_key = File::open(key_file_path).map_err(|e| HedwigError {
			error: e.to_string(),
			errcode: ErrCode::APNSPrivateKeyNotFound,
		})?;

		// Which service to call, test or production?
		let endpoint = if sandbox { Endpoint::Sandbox } else { Endpoint::Production };

		let client_config = ClientConfig::new(endpoint);

		// Connecting to APNs
		let client = Client::token(&mut private_key, key_id, team_id, client_config)
			.map_err(|e| HedwigError { error: e.to_string(), errcode: ErrCode::APNSAuthFailed })?;

		Ok(Self { client })
	}
}

#[async_trait]
impl APNSSender for APNSSenderImpl {
	async fn send(&self, payload: Payload) -> Result<(), HedwigError> {
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
