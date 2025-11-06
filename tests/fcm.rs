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
//! Tests for fcm sender.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use matrix_hedwig::{error::HedwigError, fcm::FcmSenderImpl};

#[tokio::test]
async fn fcm_sender_create() {
	let mut creds_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	creds_path.push("tests/dummy-service-account.json");

	let fcm_sender = FcmSenderImpl::new(&creds_path).await;
	let fcm_sender_debug_string = format!("{fcm_sender:?}");

	assert!(fcm_sender.is_ok());
	assert_eq!(fcm_sender_debug_string, r#"Ok(FcmSenderImpl { project_id: "dummy" })"#);
}

#[test]
fn fcm_error() {
	let error: HedwigError = gcp_auth::Error::Str("test error").into();
	let hedwig_error_debug_string = format!("{error:?}");
	assert_eq!(
		hedwig_error_debug_string,
		r#"HedwigError { error: "Failed to authenticate with fcm!", errcode: FcmAuthFailed }"#
	);
}
