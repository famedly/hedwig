//! Famedly matrix push gateway

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

#![deny(
	missing_docs,
	trivial_casts,
	trivial_numeric_casts,
	unused_extern_crates,
	unused_import_braces,
	unused_qualifications
)]
#![warn(missing_debug_implementations, dead_code, clippy::unwrap_used, clippy::expect_used)]

pub mod error;
pub mod handlers;
pub mod metrics;
pub mod models;
pub mod settings;

use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::{
	error::MatrixError,
	models::{Device, Notification},
};

/// The type of notification
#[derive(Debug)]
pub enum NotificationType {
	/// Simple notification
	Notification,
	/// Data notification
	Data,
	/// Clearing notification
	Clearing,
}

impl Display for NotificationType {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		write!(
			f,
			"{}",
			match self {
				NotificationType::Notification => "notification",
				NotificationType::Data => "data",
				NotificationType::Clearing => "clearing",
			}
		)
	}
}

/// Processed notification
#[derive(Debug)]
pub struct ProcessedNotification<'a, 'b> {
	/// The notification itself
	pub notification: &'a Notification,
	/// The first device notification is sent to
	first_device: &'a Device,
	/// App ID
	app_id: &'b str,
}

impl ProcessedNotification<'_, '_> {
	/// Some notifications may just inform the device that there are no more
	/// unread rooms
	pub fn is_clearing(&self) -> bool {
		self.notification.event_id.is_none()
			|| (!self.is_data_message() && self.unread_count() == 0)
	}

	/// Whether the push gateway should send only a data message - we have a
	/// specific app_id suffix for this
	pub fn is_data_message(&self) -> bool {
		self.first_device.app_id == format!("{}.data_message", self.app_id)
	}

	/// Determine the type of notification
	pub fn r#type(&self) -> NotificationType {
		match (self.is_clearing(), self.is_data_message()) {
			(false, true) => NotificationType::Data,
			(false, false) => NotificationType::Notification,
			(true, _) => NotificationType::Clearing,
		}
	}

	/// The number of devices a notification is sent to
	pub fn device_count(&self) -> usize {
		self.notification.devices.len()
	}

	/// List of push keys the notification is sent to
	pub fn push_keys(&self) -> Vec<&String> {
		self.notification
			.devices
			.iter()
			.filter_map(|device| {
				if device.app_id == self.app_id
					|| device.app_id == format!("{}.data_message", &self.app_id)
				{
					Some(&device.pushkey)
				} else {
					None
				}
			})
			.collect()
	}

	/// The number of unread notifications
	pub fn unread_count(&self) -> u16 {
		self.notification.counts.as_ref().and_then(|counts| counts.unread).unwrap_or_default()
	}

	/// Get the processed notification structure for easier manipulation
	pub fn process<'a, 'b>(
		push_notification: &'a models::PushNotification,
		app_id: &'b str,
	) -> Result<ProcessedNotification<'a, 'b>, MatrixError> {
		Ok(ProcessedNotification {
			first_device: push_notification.first_device()?,
			app_id,
			notification: &push_notification.notification,
		})
	}
}
