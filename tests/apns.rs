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
//! Tests for APNS sender.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use matrix_hedwig::apns::APNSSenderImpl;

#[test]
fn apns_sender_missing_key() {
	let result = APNSSenderImpl::new(
		PathBuf::from("nonexistent.key"),
		"TEAMID1234".to_owned(),
		"KEYID12345".to_owned(),
		false,
	);

	assert!(result.is_err());
	assert_eq!(result.unwrap_err().errcode, matrix_hedwig::error::ErrCode::APNSPrivateKeyNotFound);
}

#[test]
fn apns_sender_create_sandbox() {
	let result = APNSSenderImpl::new(
		PathBuf::from("tests/test.key"),
		"TEAMID1234".to_owned(),
		"KEYID12345".to_owned(),
		true,
	);

	assert!(result.is_ok());
}

#[test]
fn apns_sender_create_production() {
	let result = APNSSenderImpl::new(
		PathBuf::from("tests/test.key"),
		"TEAMID1234".to_owned(),
		"KEYID12345".to_owned(),
		false,
	);

	assert!(result.is_ok());
}
