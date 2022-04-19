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

/// The notification itself
#[derive(Deserialize, Debug)]
pub struct PushNotification {
	/// Notification
	pub notification: Notification,
}

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
	pub unread: Option<u32>,
	/// Missed calls
	pub missed_calls: Option<u32>,
}

/// The device notification is sent for
#[derive(Deserialize, Serialize, Debug)]
pub struct Device {
	/// ID of the application
	pub app_id: String,
	/// Push key
	pub pushkey: String,
	/// Timestamp of the last Push key update
	pub pushkey_ts: Option<u64>,
	/// Pusher specific data
	pub data: Option<PusherData>,
	/// A dictionary of customisations made to the way this notification is to
	/// be presented.
	pub tweaks: Option<Tweaks>,
}

/// Pusher specific data
#[derive(Deserialize, Serialize, Debug, Clone)]
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
	/// If only the counts changed, for iOS
	pub is_counts_only: Option<bool>,
}

impl Notification {
	/// Some notifications may just inform the device that counts changed
	pub fn is_counts_only(&self) -> bool {
		(self.event_id.is_none() && self.ciphertext.is_none())
			|| (self.counts.is_some() && self.unread_count() == 0)
			// if we have an encrypted push with counts_only_type = boolean
			|| self.is_counts_only == Some(true)
			// if we have an encrypted push with counts_only_type = full
			|| (self.counts.is_some() && self.ciphertext.is_some())
	}

	/// The number of devices a notification is sent to
	pub fn device_count(&self) -> usize {
		self.devices.len()
	}

	/// The number of unread notifications
	pub fn unread_count(&self) -> u32 {
		self.counts.as_ref().and_then(|counts| counts.unread).unwrap_or_default()
	}
}

/// The stripped notification to send out to the device
#[derive(Deserialize, Serialize, Debug)]
pub struct StrippedNotification {
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
	#[serde(skip_serializing_if = "Option::is_none")]
	pub prio: Option<Priority>,
	/// This is a dictionary of the current number of unacknowledged
	/// communications for the recipient user.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub counts: Option<Counts>,
	/// The content field from the event, if present.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content: Option<serde_json::Value>,
	/// The algorithm this notification has been created with, if there is any
	#[serde(skip_serializing_if = "Option::is_none")]
	pub algorithm: Option<String>,
	/// The ciphertext of an encrypted push payload
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ciphertext: Option<String>,
	/// The ephemeral key of an encrypted push payload
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ephemeral: Option<String>,
	/// The mac of an encrypted push payload
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mac: Option<String>,
}

impl StrippedNotification {
	/// Create a stripped notification from a full notification object and the
	/// specified device
	pub fn from_notif(n: &Notification, device: &Device) -> StrippedNotification {
		StrippedNotification {
			event_id: n.event_id.to_owned(),
			room_id: n.room_id.to_owned(),
			r#type: n.r#type.to_owned(),
			sender: n.sender.to_owned(),
			sender_display_name: n.sender_display_name.to_owned(),
			room_name: n.room_name.to_owned(),
			room_alias: n.room_alias.to_owned(),
			prio: n.prio.clone(),
			counts: n.counts.clone(),
			content: n.content.to_owned(),
			algorithm: device.data.clone().map(|d| d.algorithm).flatten(),
			ciphertext: n.ciphertext.to_owned(),
			ephemeral: n.ephemeral.to_owned(),
			mac: n.mac.to_owned(),
		}
	}
}

/// Response from the push gateway
#[derive(Serialize, Debug)]
pub struct PushGatewayResponse {
	/// The list of rejected notification push keys
	pub rejected: Vec<String>,
}
