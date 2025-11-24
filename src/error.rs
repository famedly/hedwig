//! Error handling structures

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
use std::fmt::{Display, Formatter, Result as FmtResult};

//use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use axum::{http::StatusCode, response::Response, Json};
use serde::Serialize;
use tracing::error;

/// Matrix error types
#[derive(Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrCode {
	/// The notification json is malformed
	BadJson,
	/// Fcm notification building/sending failure
	FcmFailed,
	/// Fcm Auth failure
	FcmAuthFailed,
	/// APNS Private Key not found
	APNSPrivateKeyNotFound,
	/// APNS Auth failure
	APNSAuthFailed,
	/// APNS notification sending failed
	APNSFailed,
	/// APNS not configured
	APNSNotConfigured,
}

/// Matrix error
#[derive(Serialize, Debug)]
pub struct HedwigError {
	/// The error text
	pub error: String,
	/// Matrix-formatted Error code
	pub errcode: ErrCode,
}

impl std::error::Error for HedwigError {}

impl Display for HedwigError {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "{}", serde_json::to_string_pretty(self).map_err(|_| std::fmt::Error)?)
	}
}

impl From<firebae_cm::Error> for HedwigError {
	fn from(err: firebae_cm::Error) -> Self {
		error!("fcm error: {}", err);
		Self {
			error: "Something went wrong while trying to interact with fcm".to_owned(),
			errcode: ErrCode::FcmFailed,
		}
	}
}

impl From<gcp_auth::Error> for HedwigError {
	fn from(err: gcp_auth::Error) -> Self {
		error!("failed to get fcm token!: {}", err);
		Self {
			error: "Failed to authenticate with push service!".to_owned(),
			errcode: ErrCode::FcmAuthFailed,
		}
	}
}

impl From<serde_json::Error> for HedwigError {
	fn from(err: serde_json::Error) -> Self {
		Self { error: err.to_string(), errcode: ErrCode::BadJson }
	}
}

impl axum::response::IntoResponse for HedwigError {
	fn into_response(self) -> Response {
		error!("Failed extracting request body {}", &self.error);
		let mut res = Json(self).into_response();
		*res.status_mut() = StatusCode::BAD_REQUEST;
		res
	}
}
