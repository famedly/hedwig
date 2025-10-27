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

use std::{fmt, fmt::Debug};

use async_trait::async_trait;
use gcp_auth::{AuthenticationManager, CustomServiceAccount};
use google_fcm1::{
	api::SendMessageRequest, hyper_rustls, hyper_util, yup_oauth2, FirebaseCloudMessaging,
};
use serde_json::Value;

use crate::error::HedwigError;

/// Trait for allowing the use of different senders for fcm messages
/// Mainly this way to make testing possible
#[async_trait]
pub trait FcmSender: Debug {
	/// Send off a message to fcm
	async fn send(
		&self,
		req: SendMessageRequest,
		fcm_auth_path: &str,
	) -> Result<String, HedwigError>;
}

/// Default implementation for FcmSender
pub struct FcmSenderImpl {
	/// The authentication manager for refreshing tokens when needed
	authentication_manager: AuthenticationManager,
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
	pub fn new(fcm_auth_path: &str) -> Result<Self, gcp_auth::Error> {
		let service_account = CustomServiceAccount::from_file(fcm_auth_path)?;
		let project_id =
			service_account.project_id().ok_or(gcp_auth::Error::NoProjectId)?.to_owned();
		let authentication_manager = AuthenticationManager::from(service_account);

		Ok(Self { authentication_manager, project_id })
	}
}

#[async_trait]
impl FcmSender for FcmSenderImpl {
	async fn send(
		&self,
		req: SendMessageRequest,
		fcm_auth_path: &str,
	) -> Result<String, HedwigError> {
		let client =
			hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
				.build(
					hyper_rustls::HttpsConnectorBuilder::new()
						.with_native_roots()
						.unwrap()
						.https_or_http()
						.enable_http1()
						.build(),
				);
		let file = std::fs::File::open(fcm_auth_path).unwrap();
		let mut fcm_auth: Value = serde_json::from_reader(file)?;
		// yup_oauth2 needs these values even though they're not used
		fcm_auth["client_secret"] = Value::String("".to_owned());
		fcm_auth["redirect_uris"] = Value::Array(vec![]);
		let secret: yup_oauth2::ApplicationSecret = serde_json::from_value(fcm_auth)?;
		let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
			secret,
			yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
		)
		.build()
		.await
		.unwrap();
		// TODO: make this a global object
		let hub = FirebaseCloudMessaging::new(client, auth);

		// Ok(client.send(message).await?)
		println!("{:?}", hub.projects().messages_send(req, "parent").doit().await?);
		Ok("".to_string())
	}
}
