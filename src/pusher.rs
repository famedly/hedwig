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

use std::collections::HashMap;

use google_fcm1::api::{
	AndroidConfig, AndroidNotification, ApnsConfig, Message, SendMessageRequest,
};
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::debug;

use crate::{
	error::{ErrCode, HedwigError},
	fcm::FcmSender,
	models::{DataMessageType, Device, Notification},
	settings::Settings,
};

/// Pushes the FCM notification to the given device
#[allow(clippy::too_many_lines)]
#[allow(clippy::unused_async)]
pub async fn push_notification(
	notification: &Notification,
	device: &Device,
	sender: &Mutex<Box<dyn FcmSender + Send + Sync>>,
	settings: &Settings,
) -> Result<(), HedwigError> {
	if !device.app_id.starts_with(&settings.hedwig.app_id) {
		return Err(HedwigError { error: "Invalid app id!".to_owned(), errcode: ErrCode::BadJson });
	}

	let count = notification.counts.as_ref().and_then(|c| c.unread).unwrap_or_default();

	let fcm_notification = google_fcm1::api::Notification {
		title: Some(settings.hedwig.fcm_notification_title.replace("<count>", &count.to_string())),
		body: Some(settings.hedwig.fcm_notification_body.clone()),
		image: None,
	};
	let mut message = Message {
		notification: None,
		android: None,
		apns: None,
		token: Some(device.pushkey.clone()),
		data: None,
		condition: None,
		fcm_options: None,
		name: None,
		topic: None,
		webpush: None,
	};

	debug!("Pushing notification to {:?} device", device.data_message_type());
	match device.data_message_type() {
		DataMessageType::Android => {
			// Used on android for background notification handling

			let mut android_config = AndroidConfig::default();
			android_config.direct_boot_ok = Some(false);
			android_config.priority = Some("high".to_string());

			message.android = Some(android_config);
			message.data =
				serde_json::from_value(serde_json::to_value(notification.data(device)?)?)?;
		}
		DataMessageType::Ios => {
			// Used for background notification handling on iOS, if enabled by the app

			// If there's no room_id then this is a badge only notification that must not
			// have any notification content
			if notification.room_id.is_some() {
				// If apple decide not to run the service extension there needs to be a fallback
				// notification
				message.notification = Some(fcm_notification);
			}

			message.data =
				serde_json::from_value(serde_json::to_value(notification.data(device)?)?)?;

			let mut ios_config = ApnsConfig::default();
			ios_config.payload = Some(HashMap::from([(
				"aps".to_string(),
				Value::Object(
					HashMap::from([
						("mutable-content".to_string(), Value::from(1)),
						("badge".to_string(), Value::from(count)),
						(
							"sound".to_owned(),
							Value::from(settings.hedwig.fcm_notification_sound.to_string()),
						),
					])
					.into_iter()
					.collect(),
				),
			)]));

			ios_config.headers = Some(HashMap::from([
				// Priority needs to be 5 for the service extension to be used
				("apns-priority".to_owned(), "5".to_owned()),
				("apns-push-type".to_owned(), settings.hedwig.fcm_apns_push_type.clone()),
			]));

			message.apns = Some(ios_config);
		}
		DataMessageType::None => {
			// Generic notification following the settings
			// This codepath runs on old versions of the iOS app also works fine with
			// android ones

			// If there's no room_id then this is a badge only notification that must not
			// have any notification content
			if notification.room_id.is_some() {
				message.notification = Some(fcm_notification);
			}

			let mut android_notification = AndroidNotification::default();
			android_notification.channel_id =
				Some(settings.hedwig.fcm_notification_android_channel_id.clone());
			android_notification.icon = Some(settings.hedwig.fcm_notification_icon.clone());
			android_notification.sound = Some(settings.hedwig.fcm_notification_sound.clone());
			android_notification.tag = Some(settings.hedwig.fcm_notification_tag.clone());
			android_notification.click_action =
				Some(settings.hedwig.fcm_notification_click_action.clone());

			let mut android_config = AndroidConfig::default();
			android_config.notification = Some(android_notification);
			android_config.direct_boot_ok = Some(false);
			android_config.priority = Some("high".to_string());

			let mut ios_config = ApnsConfig::default();
			ios_config.headers = Some(HashMap::from([
				("apns-priority".to_string(), "10".to_string()),
				("apns-push-type".to_string(), settings.hedwig.fcm_apns_push_type.clone()),
			]));
			ios_config.payload = Some(HashMap::from([(
				"aps".to_owned(),
				Value::Object(
					HashMap::from([
						("badge".to_owned(), Value::from(count)),
						(
							"sound".to_owned(),
							Value::from(settings.hedwig.fcm_notification_sound.clone()),
						),
					])
					.into_iter()
					.collect(),
				),
			)]));

			message.android = Some(android_config);
			message.apns = Some(ios_config);
		}
	};

	let req = SendMessageRequest { message: Some(message), validate_only: Some(false) };

	sender.lock().await.send(req, &settings.hedwig.fcm_service_account_token_path).await?;

	Ok(())
}
