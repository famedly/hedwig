//! JSON model structures

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

use serde::{Deserialize, Serialize};

use crate::error::{ErrCode, MatrixError};

/// The notification itself
#[derive(Deserialize, Debug)]
pub struct PushNotification {
	/// Notification
	pub notification: Notification,
}

impl PushNotification {
	/// Extract the first device from the notification list
	pub fn first_device(&self) -> Result<&Device, MatrixError> {
		self.notification.devices.first().ok_or(MatrixError {
			error: String::from("No devices were provided"),
			errcode: ErrCode::MBadJson,
		})
	}
}

/// The notification priority
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
	/// Low priority
	Low,
	/// High priority
	High,
}

/// Notification counts
#[derive(Deserialize, Serialize, Debug)]
pub struct Counts {
	/// Unread notifications
	pub unread: Option<u16>,
	/// Missed calls
	pub missed_calls: Option<u16>,
}

/// The device notification is sent for
#[derive(Deserialize, Serialize, Debug)]
pub struct Device {
	/// ID of the application
	pub app_id: String,
	/// Push key
	pub pushkey: String,
	/// Timestamp of the last Push key update
	pub pushkey_ts: Option<u32>,
	/// Pusher specific data
	pub data: Option<PusherData>,
	/// A dictionary of customisations made to the way this notification is to
	/// be presented.
	pub tweaks: Option<Tweaks>,
}

/// Pusher specific data
#[derive(Deserialize, Serialize, Debug)]
pub struct PusherData {
	/// Url to send notifications to
	pub url: Option<String>,
	/// The format to use when sending notifications to the Push Gateway.
	pub format: Option<String>,
	/// The algorithm used to potentially encrypt the push payload.
	pub algorithm: Option<String>,
}

/// A dictionary of customisations made to the way this notification is to be
/// presented.
#[derive(Deserialize, Serialize, Debug)]
pub struct Tweaks {
	/// Should sound be played
	pub sound: Option<String>,
	/// Should the message be highlighted
	pub highlight: Option<bool>,
}

/// The notification body
#[derive(Deserialize, Serialize, Debug)]
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
}

/// Response from the push gateway
#[derive(Serialize, Debug)]
pub struct PushGatewayResponse {
	/// The list of rejected notification push keys
	pub rejected: Vec<String>,
}
