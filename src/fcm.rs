//! Data structure for generic way to send messages to the real Fcm instance
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

use std::{
	env::set_var,
	fmt::{self, Debug},
	path::PathBuf,
	sync::Arc,
};

use async_trait::async_trait;
use firebae_cm::{Message, MessageBody};
use gcp_auth::TokenProvider;

use crate::error::HedwigError;

/// Trait for allowing the use of different senders for fcm messages
/// This is mainly to make testing possible
#[async_trait]
pub trait FcmSender: Debug {
	/// Send off a message to fcm
	async fn send(&self, message: MessageBody) -> Result<String, HedwigError>;
}

/// Default implementation for FcmSender
pub struct FcmSenderImpl {
	/// The authentication manager for refreshing tokens when needed
	provider: Arc<dyn TokenProvider>,
	/// The project id of the fcm project
	project_id: String,
}

impl Debug for FcmSenderImpl {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("FcmSenderImpl").field("project_id", &self.project_id).finish()
	}
}

impl FcmSenderImpl {
	/// Create new fcm sender from the path to a service-account fcm token
	pub async fn new(credentials_file_path: &PathBuf) -> Result<Self, gcp_auth::Error> {
		// gcp_auth will read this env var
		set_var("GOOGLE_APPLICATION_CREDENTIALS", credentials_file_path);
		let provider = gcp_auth::provider().await?;
		let project_id: String = provider.project_id().await?.to_string();

		Ok(Self { provider, project_id })
	}
}

#[async_trait]
impl FcmSender for FcmSenderImpl {
	async fn send(&self, body: MessageBody) -> Result<String, HedwigError> {
		let client = firebae_cm::Client::new();
		let token = self
			.provider
			.token(&["https://www.googleapis.com/auth/firebase.messaging"])
			.await
			.map(|e| e.as_str().to_owned());
		let message = Message::new(self.project_id.clone(), token?, body);

		Ok(client.send(message).await?)
	}
}
