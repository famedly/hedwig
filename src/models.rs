//! JSON model structures

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

use async_trait::async_trait;
use axum::{body::Body, extract::FromRequest, http::Request, Json};
use opentelemetry::metrics::{Counter, Histogram, Meter};
use serde::{Deserialize, Serialize};

use crate::error::{ErrCode, HedwigError};

/// The notification priority
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
	/// Low priority
	Low,
	/// High priority
	High,
}

/// Notification counts
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Counts {
	/// Unread notifications
	pub unread: Option<u16>,
	/// Missed calls
	pub missed_calls: Option<u16>,
}

/// Device data
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Data {
	/// The data message format
	pub data_message: Option<String>,
	/// The rest of the device data
	#[serde(flatten)]
	pub data: HashMap<String, serde_json::Value>,
}

/// The device notification is sent for
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Device {
	/// ID of the application
	pub app_id: String,
	/// Push key
	pub pushkey: String,
	/// Timestamp of the last Push key update
	pub pushkey_ts: Option<u64>,
	/// Pusher specific data
	pub data: Option<Data>,
	/// A dictionary of customisations made to the way this notification is to
	/// be presented.
	pub tweaks: Option<serde_json::Value>,
}

/// What kind of data message should be sent (if any)
#[derive(Debug)]
pub enum DataMessageType {
	/// No Data message
	None,
	/// Android data message
	Android,
	/// Apns data message
	Ios, // Apple would hate me for this capitalization
}

impl Device {
	/// Returns what kind of data message is wanted (if any)
	#[must_use]
	pub fn data_message_type(&self) -> DataMessageType {
		// Deprecated, use the data field!
		// TODO: remove once clients have switched to the new format
		if self.app_id.ends_with(".data_message") {
			return DataMessageType::Android;
		}

		match self.data.as_ref().and_then(|d| d.data_message.as_ref()) {
			Some(msg) if msg == "android" => DataMessageType::Android,
			Some(msg) if msg == "ios" => DataMessageType::Ios,
			_ => DataMessageType::None,
		}
	}
}

/// The notification request body
#[derive(Deserialize, Serialize, Debug)]
pub struct NotificationRequest {
	/// The actual notification
	pub notification: Notification,
}

/// The notification data
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Notification {
	/// The Matrix event ID
	pub event_id: Option<String>,
	/// The Matrix room ID
	pub room_id: Option<String>,
	/// The event type
	pub r#type: Option<String>,
	/// The sender of the event
	pub sender: Option<String>,
	/// The sender display name
	pub sender_display_name: Option<String>,
	/// The name of the room in which the event occurred.
	pub room_name: Option<String>,
	/// An alias to display for the room in which the event occurred.
	pub room_alias: Option<String>,
	/// The priority of the notification. If omitted, high is assumed.
	pub prio: Option<Priority>,
	/// This is a dictionary of the current number of unacknowledged
	/// communications for the recipient user.
	pub counts: Option<Counts>,
	/// The content field from the event, if present.
	pub content: Option<serde_json::Value>,
	/// This is an array of devices that the notification should be sent to.
	pub devices: Vec<Device>,
	/// The ciphertext of an encrypted push payload
	pub ciphertext: Option<String>,
	/// The ephemeral key of an encrypted push payload
	pub ephemeral: Option<String>,
	/// The mac of an encrypted push payload
	pub mac: Option<String>,
	/// This is true if the user receiving the notification is the subject of a
	/// member event (i.e. the state_key of the member event is equal to the
	/// user’s Matrix ID).
	pub user_is_target: Option<bool>,
}

impl Notification {
	/// Returns the data to be attached to the notification
	pub fn data(&self, device: &Device) -> Result<NotificationData, HedwigError> {
		Ok(NotificationData {
			content: serde_json::to_string(&self.content)?,
			counts: serde_json::to_string(&self.counts)?,
			// Pretending there is only one device to avoid going over any size limits
			devices: serde_json::to_string(&[device])?,
			event_id: self.event_id.clone(),
			prio: serde_json::to_string(&self.prio)?,
			room_alias: self.room_alias.clone(),
			room_id: self.room_id.clone(),
			room_name: self.room_name.clone(),
			sender: self.sender.clone(),
			sender_display_name: self.sender_display_name.clone(),
			r#type: self.r#type.clone(),
			ciphertext: self.ciphertext.clone(),
			ephemeral: self.ephemeral.clone(),
			mac: self.mac.clone(),
			user_is_target: self.user_is_target.map(|x| x.to_string()),
		})
	}
}

#[async_trait]
impl<S> FromRequest<S, Body> for Notification
where
	S: Send + Sync,
{
	type Rejection = HedwigError;

	async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
		let Json(notifcation_request) = Json::<NotificationRequest>::from_request(req, state)
			.await
			.map_err(|err| HedwigError { error: err.to_string(), errcode: ErrCode::BadJson })?;

		Ok(notifcation_request.notification)
	}
}

/// The notification data to be pushed to the client
#[derive(Deserialize, Serialize, Debug)]
pub struct NotificationData {
	/// The Matrix event ID
	#[serde(skip_serializing_if = "Option::is_none")]
	pub event_id: Option<String>,
	/// The Matrix room ID
	#[serde(skip_serializing_if = "Option::is_none")]
	pub room_id: Option<String>,
	/// The event type
	#[serde(skip_serializing_if = "Option::is_none")]
	pub r#type: Option<String>,
	/// The sender of the event
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sender: Option<String>,
	/// The sender display name
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sender_display_name: Option<String>,
	/// The name of the room in which the event occurred.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub room_name: Option<String>,
	/// An alias to display for the room in which the event occurred.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub room_alias: Option<String>,
	/// The priority of the notification. If omitted, high is assumed.
	pub prio: String,
	/// This is a dictionary of the current number of unacknowledged
	/// communications for the recipient user.
	pub counts: String,
	/// The content field from the event, if present.
	pub content: String,
	/// This is an array of devices that the notification should be sent to.
	pub devices: String,
	/// The ciphertext of an encrypted push payload
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ciphertext: Option<String>,
	/// The ephemeral key of an encrypted push payload
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ephemeral: Option<String>,
	/// The mac of an encrypted push payload
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mac: Option<String>,
	/// This is true if the user receiving the notification is the subject of a
	/// member event (i.e. the state_key of the member event is equal to the
	/// user’s Matrix ID).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub user_is_target: Option<String>,
}

/// Response from the push gateway
#[derive(Serialize, Debug)]
pub struct PushGatewayResponse {
	/// The list of rejected notification push keys
	pub rejected: Vec<String>,
}

/// Metrics for prometheus
#[derive(Debug)]
pub struct Metrics {
	/// Counter for successful pushes categorised by device type
	pub successful_pushes: Counter<u64>,
	/// Counter for failed pushes categorised by device type
	pub failed_pushes: Counter<u64>,
	/// Histogram of rolled jitter values
	pub jitter: Histogram<f64>,
}

impl Metrics {
	/// Create and register hedwigs prometheus metrics
	#[must_use]
	pub fn new(meter: &Meter) -> Self {
		Self {
			successful_pushes: meter
				.u64_counter("pushes.successful")
				.with_description("Successful pushes")
				.init(),
			failed_pushes: meter
				.u64_counter("pushes.failed")
				.with_description("Failed pushes")
				.init(),
			jitter: meter.f64_histogram("jitter").with_description("Rolled jitter delays").init(),
		}
	}
}
