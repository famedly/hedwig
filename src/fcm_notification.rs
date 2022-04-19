//! fcm notification sender

/*
 *   Matrix Hedwig
 *   Copyright (C) 2021 Famedly GmbH
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

use fcm::{Client, FcmResponse, MessageBuilder};
use tracing::debug;

use crate::{
	error::{ErrCode, MatrixError},
	metrics::NotificationCounter,
	models::{Device, Notification, StrippedNotification},
	settings::{FcmDataPusher, FcmPusher, Pusher, Settings},
};

/// process an FCM notification
pub async fn fcm_notification(
	notification: &Notification,
	device: &Device,
	config: &Settings,
	notification_counter: &NotificationCounter,
	fcm_client: &Client,
) -> Result<(), MatrixError> {
	// String representation of the unread counter with "0" as default
	let unread_count_string = notification.unread_count().to_string();

	debug!(
		"Received FCM push notification for app_id {}, registration-ID {} and unread count {}",
		&device.app_id, &device.pushkey, &unread_count_string,
	);

	let fcm_config = config
		.pushers
		.get(&device.app_id)
		.filter(|v| matches!(v, Pusher::Fcm(_) | Pusher::FcmData(_)))
		.ok_or_else(MatrixError::unknown)?;

	let admin_key = match fcm_config {
		Pusher::Fcm(FcmPusher { admin_key, .. }) | Pusher::FcmData(FcmDataPusher { admin_key }) => {
			admin_key
		}
		_ => "", /* we already error-handled these above, so they won't ever happen. This makes
		          * the compiler happy */
	};

	let mut builder = MessageBuilder::new(admin_key, &device.pushkey);

	let title = config.notification.title.replace("<count>", &unread_count_string);

	builder.priority(fcm::Priority::High);

	if let Pusher::Fcm(fcm_config) = fcm_config {
		if notification.is_counts_only() {
			let mut notification = fcm::NotificationBuilder::new();
			notification.badge(&unread_count_string);
			builder.notification(notification.finalize());
		} else {
			let mut notification = fcm::NotificationBuilder::new();
			notification.title(&title).body(&config.notification.body).badge(&unread_count_string);
			if let Some(click_action) = &fcm_config.click_action {
				notification.click_action(click_action);
			}
			if let Some(sound) = &config.notification.sound {
				notification.sound(sound);
			}
			if let Some(icon) = &fcm_config.icon {
				notification.icon(icon);
			}
			if let Some(tag) = &fcm_config.tag {
				notification.tag(tag);
			}

			builder.notification(notification.finalize());
		}
	} else {
		builder
			.data(&StrippedNotification::from_notif(notification, device))
			.map_err(|e| MatrixError { errcode: ErrCode::MBadJson, error: e.to_string() })?;
	}

	// Send the fcm message
	let results = match fcm_client.send(builder.finalize()).await {
		Ok(FcmResponse { results: Some(results), .. }) => results,
		_ => {
			notification_counter
				.with_label_values(&[&device.app_id.to_string(), "errored"])
				.inc_by(1);
			return Err(MatrixError {
				error: String::from("Invalid response from upstream push service"),
				errcode: ErrCode::MUnknown,
			});
		}
	};

	if results.first().ok_or_else(MatrixError::unknown)?.error.is_some() {
		return Err(MatrixError {
			error: String::from("Upstream rejected the token"),
			errcode: ErrCode::MUnknown,
		});
	}

	Ok(())
}
