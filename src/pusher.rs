//! Implements the logic for generating and pushing matrix notifications to a
//! given device

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

use std::sync::Arc;

use a2::{DefaultNotificationBuilder, NotificationBuilder, NotificationOptions};
use firebae_cm::{
	self, AndroidConfig, AndroidMessagePriority, AndroidNotification, ApnsConfig, MessageBody,
};
use serde_json::json;
use tokio::sync::Mutex;
use tracing::debug;

use crate::{
	apns::APNSSender,
	error::{ErrCode, HedwigError},
	fcm::FcmSender,
	models::{ApnsHeaders, DataMessageType, Device, Notification},
	settings::Settings,
};

/// Pushes the FCM notification to the given device
#[allow(clippy::unused_async)]
pub async fn push_notification_fcm(
	notification: &Notification,
	device: &Device,
	sender: &Mutex<Box<dyn FcmSender + Send + Sync>>,
	settings: &Settings,
) -> Result<(), HedwigError> {
	if !device.app_id.starts_with(&settings.hedwig.app_id) {
		return Err(HedwigError { error: "Invalid app id!".to_owned(), errcode: ErrCode::BadJson });
	}

	let count = notification.counts.as_ref().and_then(|c| c.unread).unwrap_or_default();

	let fcm_notification = firebae_cm::Notification {
		title: Some(settings.hedwig.notification_title.replace("<count>", &count.to_string())),
		body: Some(settings.hedwig.notification_body.clone()),
		image: None,
	};

	let receiver = firebae_cm::Receiver::Token(device.pushkey.clone());
	let mut body = MessageBody::new(receiver);

	debug!("Pushing notification to {:?} device", device.data_message_type());

	match device.data_message_type() {
		DataMessageType::Android => {
			// Used on android for background notification handling

			let mut android_config = AndroidConfig::new();
			android_config.direct_boot_ok(false);
			android_config.priority(AndroidMessagePriority::High);

			body.data(notification.data(device)?)?.android(android_config);
		}
		DataMessageType::None => {
			// Generic notification following the settings
			// This codepath runs on old versions of the iOS app also works fine with
			// android ones

			// If there's no room_id then this is a badge only notification that must not
			// have any notification content
			if notification.room_id.is_some() {
				body.notification(fcm_notification);
			}

			let mut android_notification = AndroidNotification::new();
			android_notification
				.channel_id(settings.hedwig.fcm_notification_android_channel_id.clone());
			android_notification.icon(settings.hedwig.notification_icon.clone());
			android_notification.sound(settings.hedwig.notification_sound.clone());
			android_notification.tag(settings.hedwig.notification_tag.clone());
			android_notification.click_action(settings.hedwig.notification_click_action.clone());

			let mut android_config = AndroidConfig::new();
			android_config.notification(android_notification);
			android_config.direct_boot_ok(false);
			android_config.priority(AndroidMessagePriority::High);

			let mut ios_config = ApnsConfig::new();
			ios_config.headers(ApnsHeaders {
				apns_priority: "10".to_owned(),
				apns_push_type: settings.hedwig.apns_push_type.0.to_string(),
			})?;
			ios_config.payload(json!({
				"aps": {
					"badge": count,
					"sound": settings.hedwig.notification_sound
				}
			}))?;

			body.android(android_config);
			body.apns(ios_config);
		}
		DataMessageType::Ios => {
			// Used for background notification handling on iOS, if enabled by the app

			// If there's no room_id then this is a badge only notification that must not
			// have any notification content
			if notification.room_id.is_some() {
				// If apple decide not to run the service extension there needs to be a fallback
				// notification
				body.notification(fcm_notification);
			}

			body.data(notification.data(device)?)?;

			let mut ios_config = ApnsConfig::new();
			ios_config.payload(json!({
				"aps": {
					"mutable-content": 1,
					"badge": count,
					"sound": settings.hedwig.notification_sound
				}
			}))?;

			// Priority needs to be 5 for the service extension to be used
			ios_config.headers(ApnsHeaders {
				apns_priority: "5".to_owned(),
				apns_push_type: settings.hedwig.apns_push_type.0.to_string(),
			})?;

			body.apns(ios_config);
		}
	};

	sender.lock().await.send(body).await?;

	Ok(())
}

/// Pushes a notification to an iOS device using APNs
pub async fn push_notification_apns(
	notification: &Notification,
	device: &Device,
	sender: &Arc<dyn APNSSender + Send + Sync>,
	settings: &Settings,
) -> Result<(), HedwigError> {
	if !device.app_id.starts_with(&settings.hedwig.app_id) {
		return Err(HedwigError { error: "Invalid app id!".to_owned(), errcode: ErrCode::BadJson });
	}

	let count = notification.counts.as_ref().and_then(|c| c.unread).unwrap_or_default();

	let builder = DefaultNotificationBuilder::new()
		.set_body(settings.hedwig.notification_body.clone())
		.set_sound(settings.hedwig.notification_sound.clone())
		.set_title(settings.hedwig.notification_title.clone())
		.set_badge(u32::from(count))
		.set_mutable_content();

	let options = NotificationOptions {
		apns_topic: Some(sender.get_topic().to_owned()),
		apns_push_type: Some(sender.get_push_type().to_owned()),
		..Default::default()
	};

	let payload = builder.build(device.pushkey.clone(), options);

	debug!("Pushing notification to {:?} device", device.data_message_type());

	sender.send(payload).await?;

	Ok(())
}
