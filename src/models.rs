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

use actix_web::{http, web, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Deserialize, Debug)]
pub struct PushNotification {
    pub notification: Notification,
}

impl PushNotification {
    /// Collect the pushkeys and check if the app_id of each device matches
    pub fn pushkeys_for_app_id(&self, app_id: &String) -> Vec<&String> {
        self.notification
            .devices
            .iter()
            .filter_map(|device| {
                if &device.app_id == app_id || device.app_id == format!("{}.data_message", app_id) {
                    Some(&device.pushkey)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn notification_count(&self) -> u16 {
        self.notification
            .counts
            .as_ref()
            .and_then(|counts| counts.unread)
            .unwrap_or(0)
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    High,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrCode {
    MBadJson,
    MMissingParam,
    MUnknown,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Counts {
    pub unread: Option<u16>,
    pub missed_calls: Option<u16>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Device {
    pub app_id: String,
    pub pushkey: String,
    pub pushkey_ts: Option<u32>,
    pub data: Option<PusherData>,
    pub tweaks: Option<Tweaks>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PusherData {
    pub url: Option<String>,
    pub format: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Tweaks {
    pub sound: Option<String>,
    pub highlight: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Notification {
    pub event_id: Option<String>,
    pub room_id: Option<String>,
    pub r#type: Option<String>,
    pub sender: Option<String>,
    pub sender_display_name: Option<String>,
    pub room_name: Option<String>,
    pub room_alias: Option<String>,
    pub prio: Option<Priority>,
    pub counts: Option<Counts>,
    pub content: Option<serde_json::Value>,
    pub devices: Vec<Device>,
}

#[derive(Serialize, Debug)]
pub struct MatrixError {
    pub error: String,
    pub errcode: ErrCode,
}

impl Display for MatrixError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", serde_json::to_string_pretty(self).unwrap())
    }
}

impl ResponseError for MatrixError {
    fn error_response(&self) -> web::HttpResponse {
        let status_code = match self.errcode {
            ErrCode::MUnknown => http::StatusCode::BAD_GATEWAY,
            _ => http::StatusCode::BAD_REQUEST,
        };
        web::HttpResponse::build(status_code).json(self)
    }
}

#[derive(Serialize)]
pub struct PushGatewayResponse {
    pub rejected: Vec<String>,
}
