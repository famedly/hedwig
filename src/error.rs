//! Error handling structures

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
use std::fmt::{Display, Formatter, Result as FmtResult};

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use tracing::error;

/// Matrix error types
#[derive(Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrCode {
	/// The notification json is malformed
	MBadJson,
	/// The notification json is missing some parameters
	MMissingParam,
	/// Generic error
	MUnknown,
}

/// Matrix error
#[derive(Serialize, Debug)]
pub struct MatrixError {
	/// The error text
	pub error: String,
	/// Matrix-formatted Error code
	pub errcode: ErrCode,
}

impl MatrixError {
	/// Generate a generic error code
	pub fn unknown() -> Self {
		Self { error: String::from("Internal server error"), errcode: ErrCode::MUnknown }
	}
}

impl Display for MatrixError {
	fn fmt(&self, f: &mut Formatter) -> FmtResult {
		write!(f, "{}", serde_json::to_string_pretty(self).map_err(|_| std::fmt::Error::default())?)
	}
}

impl ResponseError for MatrixError {
	fn error_response(&self) -> HttpResponse {
		error!("{}", &self.error);
		let status_code = match self.errcode {
			ErrCode::MUnknown => StatusCode::BAD_GATEWAY,
			_ => StatusCode::BAD_REQUEST,
		};
		HttpResponse::build(status_code).json(self)
	}
}
