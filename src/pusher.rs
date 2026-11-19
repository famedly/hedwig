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

use a2::{DefaultNotificationBuilder, NotificationBuilder, NotificationOptions, PushType};
use firebae_cm::{
	self, AndroidConfig, AndroidMessagePriority, AndroidNotification, ApnsConfig, MessageBody,
};
use serde_json::json;
use tokio::sync::Mutex;
use tracing::debug;
use uuid::Uuid;

use crate::{
	apns::APNSSender,
	error::{ErrCode, HedwigError},
	fcm::FcmSender,
	models::{DataMessageType, Device, Notification},
	settings::Settings,
};

/// Validates a device's `app_id`, using `hedwig.voip_app_id` for VoIP
/// devices and `hedwig.app_id` otherwise.
fn validate_app_id(device: &Device, settings: &Settings) -> Result<(), HedwigError> {
	let is_voip = matches!(device.data_message_type(), DataMessageType::IosVoip);
	let allowed_prefix =
		if is_voip { &settings.hedwig.voip_app_id } else { &settings.hedwig.app_id };

	if !device.app_id.starts_with(allowed_prefix) {
		return Err(HedwigError { error: "Invalid app id!".to_owned(), errcode: ErrCode::BadJson });
	}

	Ok(())
}

/// Builds the Android notification for generic FCM pushes from the
/// configured `notification_android` settings, setting only the values that
/// are not `None`
fn android_notification(settings: &Settings) -> Result<AndroidNotification, HedwigError> {
	let config = &settings.hedwig.notification_android;
	let mut android_notification = AndroidNotification::new()
		.channel_id(config.channel_id.clone())
		.icon(config.icon.clone())
		.sound(settings.hedwig.notification_sound.clone())
		.tag(config.tag.clone())
		.click_action(settings.hedwig.notification_click_action.clone());

	if let Some(v) = &config.color {
		android_notification = android_notification.color(v.clone());
	}
	if let Some(v) = &config.body_loc_key {
		android_notification = android_notification.body_loc_key(v.clone());
	}
	if let Some(v) = &config.body_loc_args {
		android_notification = android_notification.body_loc_args(v.clone());
	}
	if let Some(v) = &config.title_loc_key {
		android_notification = android_notification.title_loc_key(v.clone());
	}
	if let Some(v) = &config.title_loc_args {
		android_notification = android_notification.title_loc_args(v.clone());
	}
	if let Some(v) = &config.ticker {
		android_notification = android_notification.ticker(v.clone());
	}
	if let Some(v) = config.event_time {
		android_notification = android_notification.event_time(v)?;
	}
	if let Some(v) = config.default_sound {
		android_notification = android_notification.default_sound(v);
	}
	if let Some(v) = &config.vibrate_timings {
		android_notification = android_notification.vibrate_timings(v.iter().copied());
	}
	if let Some(v) = &config.image {
		android_notification = android_notification.image(v.clone());
	}
	if let Some(v) = config.sticky {
		android_notification = android_notification.sticky(v);
	}
	if let Some(v) = config.local_only {
		android_notification = android_notification.local_only(v);
	}
	if let Some(v) = config.default_vibrate_timings {
		android_notification = android_notification.default_vibrate_timings(v);
	}
	if let Some(v) = config.default_light_settings {
		android_notification = android_notification.default_light_settings(v);
	}
	if let Some(v) = config.notification_priority {
		android_notification = android_notification.notification_priority(v);
	}
	if let Some(v) = config.visibility {
		android_notification = android_notification.visibility(v);
	}
	if let Some(v) = &config.light_settings {
		android_notification = android_notification.light_settings(v.clone());
	}

	Ok(android_notification)
}

/// The `aps` dictionary for iOS pushes sent through FCM
fn aps(count: u16, settings: &Settings) -> serde_json::Value {
	let mut aps = json!({
		"badge": count,
		"sound": settings.hedwig.notification_sound
	});

	// this is set dynamically as a null value will cause APNS to
	// error
	if let Some(ref category) = settings.hedwig.apns_payload.category {
		aps["category"] = json!(category);
	}
	if let Some(ref content_available) = settings.hedwig.apns_payload.content_available {
		aps["content-available"] = json!(content_available);
	}
	if let Some(ref mutable_content) = settings.hedwig.apns_payload.mutable_content {
		aps["mutable-content"] = json!(mutable_content);
	}
	aps
}

/// Pushes the FCM notification to the given device
#[allow(clippy::unused_async)]
pub async fn push_notification_fcm(
	notification: &Notification,
	device: &Device,
	sender: &Mutex<Box<dyn FcmSender + Send + Sync>>,
	settings: &Settings,
) -> Result<(), HedwigError> {
	validate_app_id(device, settings)?;

	let count = notification.counts.as_ref().and_then(|c| c.unread).unwrap_or_default();

	let fcm_notification = firebae_cm::Notification {
		title: Some(settings.hedwig.notification_title.replace("<count>", &count.to_string())),
		body: Some(
			settings.hedwig.notification_body.clone().replace("<count>", &count.to_string()),
		),
		image: None,
	};

	let receiver = firebae_cm::Receiver::token(device.pushkey.clone());
	let mut body = MessageBody::new(receiver);

	debug!("Pushing notification to {:?} device", device.data_message_type());

	match device.data_message_type() {
		DataMessageType::Android => {
			// Used on android for background notification handling

			let android_config =
				AndroidConfig::new().direct_boot_ok(false).priority(AndroidMessagePriority::High);

			body = body.data(notification.data(device)?)?.android(android_config);
		}
		DataMessageType::None => {
			// Generic notification following the settings
			// This codepath runs on old versions of the iOS app also works fine
			// with android ones

			// If there's no room_id then this is a badge only notification that
			// must not have any notification content
			if notification.room_id.is_some() {
				body = body.notification(fcm_notification);
			}

			let android_config = AndroidConfig::new()
				.notification(android_notification(settings)?)
				.direct_boot_ok(false)
				.priority(AndroidMessagePriority::High);

			let ios_config = ApnsConfig::new()
				.headers(settings.hedwig.apns_headers.clone())?
				.payload(json!({ "aps": aps(count, settings) }));

			body = body.android(android_config).apns(ios_config);
		}
		DataMessageType::Ios => {
			// Used for background notification handling on iOS, if enabled by
			// the app

			// If there's no room_id then this is a badge only notification that
			// must not have any notification content
			if notification.room_id.is_some() {
				// If apple decide not to run the service extension there needs
				// to be a fallback notification
				body = body.notification(fcm_notification);
			}

			let ios_config = ApnsConfig::new()
				.payload(json!({ "aps": aps(count, settings) }))
				.headers(settings.hedwig.apns_headers.clone())?;

			body = body.data(notification.data(device)?)?.apns(ios_config);
		}
		DataMessageType::IosVoip => {
			// VoIP pushes are routed through push_notification_voip_apns, not
			// FCM.
			return Err(HedwigError {
				error: "VoIP pushes must use the APNs sender, not FCM".to_owned(),
				errcode: ErrCode::VoipNotSupported,
			});
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
	validate_app_id(device, settings)?;

	let count = notification.counts.as_ref().and_then(|c| c.unread).unwrap_or_default();

	let mut builder = DefaultNotificationBuilder::new()
		.set_body(settings.hedwig.notification_body.clone().replace("<count>", &count.to_string()))
		.set_sound(settings.hedwig.notification_sound.clone())
		.set_title(
			settings.hedwig.notification_title.clone().replace("<count>", &count.to_string()),
		)
		.set_badge(u32::from(count));

	if settings.hedwig.apns_payload.mutable_content.is_some_and(|v| v == 1) {
		builder = builder.set_mutable_content();
	}
	if settings.hedwig.apns_payload.content_available.is_some_and(|v| v == 1) {
		builder = builder.set_content_available();
	}
	if let Some(category) = settings.hedwig.apns_payload.category.clone() {
		builder = builder.set_category(category);
	}

	let options = NotificationOptions {
		apns_topic: settings.hedwig.apns_headers.apns_topic.clone(),
		apns_push_type: Some(settings.hedwig.apns_headers.apns_push_type.0),
		..Default::default()
	};

	let payload = builder.build(device.pushkey.clone(), options);

	debug!("Pushing notification to {:?} device", device.data_message_type());

	sender.send(payload).await?;

	Ok(())
}

/// Pushes a VoIP push notification to an iOS device via APNs / PushKit
///
/// VoIP pushes use `apns-push-type: voip` and carry a raw payload (no `aps`
/// alert) with call metadata that the iOS `AppDelegate` reads to present the
/// native CallKit incoming-call screen.
pub async fn push_notification_voip_apns(
	notification: &Notification,
	device: &Device,
	sender: &Arc<dyn APNSSender + Send + Sync>,
	settings: &Settings,
) -> Result<(), HedwigError> {
	validate_app_id(device, settings)?;

	// The APNs topic must match the PushKit registration (bundle-id + ".voip")
	if !device.app_id.ends_with(".voip") {
		return Err(HedwigError {
			error: "VoIP pushes require an app_id ending in '.voip'".to_owned(),
			errcode: ErrCode::BadJson,
		});
	}

	let options = NotificationOptions {
		apns_topic: Some(device.app_id.clone()),
		apns_push_type: Some(PushType::Voip),
		..Default::default()
	};

	let mut payload = DefaultNotificationBuilder::new().build(device.pushkey.clone(), options);

	// Caller display name: prefer sender_display_name, fall back to room_name
	let name = notification
		.sender_display_name
		.as_deref()
		.or(notification.room_name.as_deref())
		.unwrap_or("Unknown");

	// Stable UUID derived from event_id so CallKit can deduplicate; falls back
	// to a random UUID when the homeserver sends event_id_only format without
	// an id. The id must be a valid UUID string (the app parses it with
	// UUID(uuidString:)), so the Matrix event_id ($abc...) is hashed into a
	// UUIDv5 rather than passed raw.
	let call_id = notification.event_id.as_ref().map_or_else(
		|| Uuid::new_v4().to_string(),
		|event_id| Uuid::new_v5(&Uuid::NAMESPACE_OID, event_id.as_bytes()).to_string(),
	);

	// Fields read by AppDelegate via PKPushPayload.dictionaryPayload
	payload.add_custom_data("id".to_owned(), &call_id)?;
	// Extra `&` needed: `add_custom_data` takes `&dyn Serialize`, and `str` is
	// unsized.
	payload.add_custom_data("nameCaller".to_owned(), &name)?;
	// No caller address is known at push time, so the CallKit handle is
	// intentionally left empty; the app resolves the call via the id.
	payload.add_custom_data("handle".to_owned(), &"")?;
	// The call type is not known at push time (event content is encrypted), so
	// default to an audio call; the app updates the UI after decryption.
	payload.add_custom_data("isVideo".to_owned(), &false)?;

	debug!("Pushing VoIP notification to {:?} device", device.data_message_type());

	sender.send(payload).await?;

	Ok(())
}
