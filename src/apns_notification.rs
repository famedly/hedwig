//! apns notification sender

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

use std::collections::HashMap;

use a2::{
	Client, LocalizedNotificationBuilder, NotificationBuilder, NotificationOptions,
	PlainNotificationBuilder, Priority, SilentNotificationBuilder,
};
use tracing::debug;

use crate::{
	error::{ErrCode, MatrixError},
	metrics::NotificationCounter,
	models::{Device, Notification, StrippedNotification},
	settings::{Pusher, Settings},
};

/// process an APNS notification
pub async fn apns_notification(
	notification: &Notification,
	device: &Device,
	config: &Settings,
	notification_counter: &NotificationCounter,
	apns_clients: &HashMap<String, Client>,
) -> Result<(), MatrixError> {
	// String representation of the unread counter with "0" as default
	let unread_count_string = notification.unread_count().to_string();

	let apns_config = config
		.pushers
		.get(&device.app_id)
		.map(|v| match v {
			Pusher::Apns(config) => Some(config),
			_ => None,
		})
		.flatten()
		.ok_or_else(MatrixError::unknown)?;

	let client = apns_clients.get(&device.app_id).ok_or_else(MatrixError::unknown)?;

	debug!(
		"Received APNS push notification for app_id {}, registration-ID {} and unread count {}",
		&device.app_id, &device.pushkey, &unread_count_string,
	);

	let options = NotificationOptions {
		apns_topic: apns_config.topic.as_deref(),
		apns_priority: Some(Priority::High),
		..Default::default()
	};

	let mut payload;

	let title = config.notification.title.replace("<count>", &unread_count_string);

	if notification.is_counts_only() {
		if notification.counts.is_some() {
			// we send the badge message
			let mut builder = PlainNotificationBuilder::new("");
			builder.set_badge(notification.unread_count());
			payload = builder.build(device.pushkey.as_ref(), options);
		} else {
			// we send a background update
			payload = SilentNotificationBuilder::new().build(device.pushkey.as_ref(), options);
		}
	} else {
		let mut builder = LocalizedNotificationBuilder::new(&title, &config.notification.body);
		builder.set_mutable_content();
		builder.set_badge(notification.unread_count());
		builder.set_sound("default");
		if let Some(sound) = &config.notification.sound {
			builder.set_sound(sound);
		}
		payload = builder.build(device.pushkey.as_ref(), options);
	}

	payload
		.add_custom_data("data", &StrippedNotification::from_notif(notification, device))
		.map_err(|_| MatrixError {
			error: String::from("Failed to add data to apns payload"),
			errcode: ErrCode::MUnknown,
		})?;
	let response = client.send(payload).await.map_err(|_| {
		notification_counter.with_label_values(&[&device.app_id.to_string(), "errored"]).inc_by(1);
		MatrixError {
			error: String::from("Invalid response from upstream push service"),
			errcode: ErrCode::MUnknown,
		}
	})?;

	if response.error.is_some() {
		return Err(MatrixError {
			error: String::from("Upstream rejected the token"),
			errcode: ErrCode::MUnknown,
		});
	}

	Ok(())
}
