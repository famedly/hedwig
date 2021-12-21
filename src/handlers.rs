//! http request handlers

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

use std::{collections::HashMap, convert::TryFrom, sync::Arc};

use axum::{
	async_trait,
	extract::{rejection::JsonRejection, Extension, FromRequest, RequestParts},
	http::StatusCode,
	BoxError,
};
use serde::de::DeserializeOwned;
use tracing::debug;

use crate::{
	apns_notification::apns_notification,
	error::{ErrCode, MatrixError},
	fcm_notification::fcm_notification,
	metrics::{DeviceCounter, LastSuccessfulCollector, NotificationCounter},
	models::{PushGatewayResponse, PushNotification},
	settings::{Pusher, Settings},
};

/// Json deserializer with matrix errors
#[derive(Debug)]
pub struct Json<T>(T);

#[async_trait]
impl<B, T> FromRequest<B> for Json<T>
where
	// these trait bounds are copied from `impl FromRequest for
	// axum::Json`
	T: DeserializeOwned,
	B: axum::body::HttpBody + Send,
	B::Data: Send,
	B::Error: Into<BoxError>,
{
	type Rejection = (StatusCode, MatrixError);

	async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
		match axum::Json::<T>::from_request(req).await {
			Ok(value) => Ok(Self(value.0)),
			Err(rejection) => {
				// convert the error from `axum::Json` into whatever we want
				match rejection {
					JsonRejection::InvalidJsonBody(err) => Err((
						StatusCode::BAD_REQUEST,
						MatrixError {
							error: format!("Invalid JSON request: {}", err),
							errcode: ErrCode::MBadJson,
						},
					)),
					JsonRejection::MissingJsonContentType(err) => Err((
						StatusCode::BAD_REQUEST,
						MatrixError { error: err.to_string(), errcode: ErrCode::MBadJson },
					)),

					JsonRejection::HeadersAlreadyExtracted(err) => Err((
						StatusCode::INTERNAL_SERVER_ERROR,
						MatrixError { error: err.to_string(), errcode: ErrCode::MUnknown },
					)),
					err => Err((
						StatusCode::INTERNAL_SERVER_ERROR,
						MatrixError {
							error: format!("Unknown internal error: {}", err),
							errcode: ErrCode::MUnknown,
						},
					)),
				}
			}
		}
	}
}

/// process the notification
pub async fn process_notification(
	Json(push_notification): Json<PushNotification>,
	config: Extension<Settings>,
	notification_counter: Extension<NotificationCounter>,
	last_notification_gauge: Extension<LastSuccessfulCollector>,
	device_counter: Extension<DeviceCounter>,
	fcm_client: Extension<Arc<fcm::Client>>,
	apns_clients: Extension<Arc<HashMap<String, a2::Client>>>,
) -> Result<axum::Json<PushGatewayResponse>, MatrixError> {
	let notification = &push_notification.notification;

	device_counter
		.with_label_values(&[])
		.inc_by(u64::try_from(notification.device_count()).map_err(|_| MatrixError::unknown())?);

	let mut rejected: Vec<String> = Vec::new();
	let mut succeeded: Vec<String> = Vec::new();
	for device in notification.devices.iter() {
		let push_key = device.pushkey.to_owned();
		let app_id = device.app_id.to_owned();

		let result = match config.pushers.get(&app_id) {
			Some(Pusher::Fcm { .. } | Pusher::FcmData { .. }) => {
				fcm_notification(notification, device, &config, &notification_counter, &fcm_client)
					.await
			}
			Some(Pusher::Apns { .. }) => {
				apns_notification(
					notification,
					device,
					&config,
					&notification_counter,
					&apns_clients,
				)
				.await
			}
			None => Err(MatrixError {
				error: String::from("Invalid app id"),
				errcode: ErrCode::MUnknown,
			}),
		};

		if let Err(err) = result {
			debug!(
				"Rejecting app id {} push key {} with reason {}",
				&app_id, &push_key, &err.error
			);
			rejected.push(push_key);
			notification_counter.with_label_values(&[&app_id, "rejected"]).inc_by(1);
		} else {
			succeeded.push(push_key);
			notification_counter.with_label_values(&[&app_id, "succeeded"]).inc_by(1);
		}
	}

	if !succeeded.is_empty() {
		last_notification_gauge.update();
	}

	debug!("Rejected: {:?}", &rejected);
	debug!("Succeeded: {:?}", &succeeded);

	Ok(axum::Json(PushGatewayResponse { rejected }))
}
