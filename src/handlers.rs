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

use std::convert::{TryFrom, TryInto};

use actix_web::{error::JsonPayloadError, web, HttpRequest, HttpResponse};
use fcm::{FcmResponse, Priority};
use tracing::debug;

use crate::{
	error::{ErrCode, MatrixError},
	metrics::{DeviceCounter, LastSuccessfulCollector, NotificationCounter},
	models::{PushGatewayResponse, PushNotification},
	settings::Settings,
	NotificationType, ProcessedNotification,
};

/// Handle json parsing error
pub fn json_error_handler(err: JsonPayloadError, _: &HttpRequest) -> actix_web::Error {
	if let JsonPayloadError::Deserialize(deserialize_err) = &err {
		if deserialize_err.classify() == serde_json::error::Category::Data {
			return MatrixError {
				error: deserialize_err.to_string(),
				errcode: ErrCode::MMissingParam,
			}
			.into();
		}
	}
	MatrixError { error: err.to_string(), errcode: ErrCode::MBadJson }.into()
}

/// process the notification
pub async fn process_notification(
	push_notification: web::Json<PushNotification>,
	config: web::Data<Settings>,
	notification_counter: web::Data<NotificationCounter>,
	last_notification_gauge: web::Data<LastSuccessfulCollector>,
	device_counter: web::Data<DeviceCounter>,
	fcm_client: web::Data<fcm::Client>,
) -> Result<HttpResponse, MatrixError> {
	let processed_notification =
		ProcessedNotification::process(&push_notification, &config.hedwig.app_id)?;
	let registration_ids = processed_notification.push_keys();
	if registration_ids.is_empty() {
		return Err(MatrixError {
			error: String::from("No registration IDs that match the app ID were provided"),
			errcode: ErrCode::MBadJson,
		});
	}
	device_counter.with_label_values(&[]).inc_by(
		u64::try_from(processed_notification.device_count()).map_err(|_| MatrixError::unknown())?,
	);

	// String representation of the unread counter with "0" as default
	let unread_count_string = processed_notification.unread_count().to_string();

	debug!(
		"Received push notification for registration-ID(s) {:?}, message type = {} and unread count {}",
		&registration_ids,
		&processed_notification.r#type(),
		&unread_count_string
	);

	// Get the MessageBuilder - either we need to send the notification to one or to
	// multiple devices
	let mut builder = if registration_ids.len() == 1 {
		fcm::MessageBuilder::new(
			&config.hedwig.fcm_admin_key,
			registration_ids.first().ok_or_else(MatrixError::unknown)?,
		)
	} else {
		fcm::MessageBuilder::new_multi(&config.hedwig.fcm_admin_key, &registration_ids)
	};

	// Set the data for fcm here
	builder.priority(Priority::High);
	builder
		.data(&processed_notification.notification)
		.map_err(|e| MatrixError { errcode: ErrCode::MBadJson, error: e.to_string() })?;

	// Set additional keys for the notification message

	let title = config.hedwig.fcm_notification_title.replace("<count>", &unread_count_string);
	if let NotificationType::Notification = processed_notification.r#type() {
		let mut notification = fcm::NotificationBuilder::new();
		notification
			.title(&title)
			.click_action(&config.hedwig.fcm_notification_click_action)
			.body(&config.hedwig.fcm_notification_body)
			.badge(&unread_count_string)
			.sound(&config.hedwig.fcm_notification_sound)
			.icon(&config.hedwig.fcm_notification_icon)
			.tag(&config.hedwig.fcm_notification_tag);

		builder.notification(notification.finalize());
	} else if processed_notification.is_clearing() && !processed_notification.is_data_message() {
		let mut notification = fcm::NotificationBuilder::new();
		notification.badge(&unread_count_string);
		builder.notification(notification.finalize());
	}

	// Send the fcm message
	let results = match fcm_client.send(builder.finalize()).await {
		Ok(FcmResponse { results: Some(results), .. }) => results,
		_ => {
			notification_counter
				.with_label_values(&[&processed_notification.r#type().to_string(), "errored"])
				.inc_by(registration_ids.len().try_into().map_err(|_| MatrixError::unknown())?);
			return Err(MatrixError {
				error: String::from("Invalid response from upstream push service"),
				errcode: ErrCode::MUnknown,
			});
		}
	};
	let rejected: Vec<String> = results
		.iter()
		.enumerate()
		.filter_map(|(idx, result)| {
			result.error.and_then(|_| registration_ids.get(idx).cloned().cloned())
		})
		.collect();

	let succeeded: Vec<String> = results
		.iter()
		.enumerate()
		.filter_map(|(idx, result)| {
			if result.error.is_none() {
				registration_ids.get(idx).cloned().cloned()
			} else {
				None
			}
		})
		.collect();

	if !succeeded.is_empty() {
		last_notification_gauge.update();
	}

	debug!("Rejected: {:?}", &rejected);
	debug!("Succeeded: {:?}", &succeeded);

	notification_counter
		.with_label_values(&[&processed_notification.r#type().to_string(), "rejected"])
		.inc_by(rejected.len().try_into().map_err(|_| MatrixError::unknown())?);
	notification_counter
		.with_label_values(&[&processed_notification.r#type().to_string(), "succeeded"])
		.inc_by(succeeded.len().try_into().map_err(|_| MatrixError::unknown())?);

	Ok(HttpResponse::Ok().json(&PushGatewayResponse { rejected }))
}
