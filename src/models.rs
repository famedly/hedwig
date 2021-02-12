use actix_web::{http, web, ResponseError};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Deserialize, Debug)]
pub struct PushNotification {
    pub notification: Notification,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    High,
}

#[derive(Serialize, Debug)]
pub enum ErrCode {
    M_BAD_JSON,
    M_MISSING_PARAM,
    M_UNKNOWN,
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
            ErrCode::M_UNKNOWN => http::StatusCode::BAD_GATEWAY,
            _ => http::StatusCode::BAD_REQUEST,
        };
        web::HttpResponse::build(status_code).json(self)
    }
}

#[derive(Serialize)]
pub struct PushGatewayResponse {
    pub rejected: Vec<String>,
}
