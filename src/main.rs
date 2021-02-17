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

use actix_web::{web, App, HttpResponse, HttpServer};
mod models;
mod settings;
use actix_web::error::JsonPayloadError;
use fcm::{FcmResponse, Priority};
use models::{ErrCode, MatrixError, PushGatewayResponse, PushNotification};
use settings::Settings;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

async fn process_notification(
    push_notification: web::Json<PushNotification>,
    config: web::Data<Settings>,
    fcm_client: web::Data<fcm::Client>,
) -> Result<HttpResponse, MatrixError> {
    info!("========> NEW PUSH NOTIFICATION RECEIVED <========");
    let registration_ids = push_notification.pushkeys_for_app_id(&config.app_id);

    if registration_ids.is_empty() {
        return Err(MatrixError {
            error: String::from("No registration IDs with the matching app ID provided"),
            errcode: ErrCode::MBadJson,
        });
    }

    let first_device = push_notification
        .notification
        .devices
        .first()
        .ok_or(MatrixError {
            error: String::from("No devices were provided"),
            errcode: ErrCode::MBadJson,
        })?;

    info!("Registration IDs: {:?}", &registration_ids);

    // Whether the push gateway should send only a data message - we have a specific app_id suffix for this
    let data_message_mode = first_device.app_id == format!("{}.data_message", config.app_id);
    info!("Data message mode: {}", &data_message_mode);

    // Some notifications may just inform the device that there are no more unread rooms
    let is_clearing_notification = push_notification.notification.event_id.is_none();
    info!("Is clearing notification: {}", &is_clearing_notification);

    // String representation of the unread counter with "0" as default
    let unread_count_string = push_notification.notification_count().to_string();
    info!("unread count: {}", &unread_count_string);

    // Get the MessageBuilder - either we need to send the notification to one or to multiple devices
    let mut builder = if registration_ids.len() == 1 {
        fcm::MessageBuilder::new(&config.fcm_admin_key, registration_ids.first().unwrap())
    } else {
        fcm::MessageBuilder::new_multi(&config.fcm_admin_key, &registration_ids)
    };

    // Set the data for fcm here
    builder.collapse_key(&config.fcm_collapse_key);
    builder.priority(Priority::High);
    builder
        .data(&push_notification.notification)
        .map_err(|e| MatrixError {
            errcode: ErrCode::MBadJson,
            error: e.to_string(),
        })?;

    // Set additional keys for the notification message
    let title = config
        .fcm_notification_title
        .replace("<count>", &unread_count_string);
    if !data_message_mode && !is_clearing_notification {
        let mut notification = fcm::NotificationBuilder::new();
        notification
            .title(&title)
            .click_action(&config.fcm_notification_click_action)
            .body(&config.fcm_notification_body)
            .badge(&unread_count_string)
            .sound(&config.fcm_notification_sound)
            .icon(&config.fcm_notification_icon)
            .tag(&config.fcm_notification_tag);

        builder.notification(notification.finalize());
    }

    // Send the fcm message
    if let Ok(FcmResponse {
        results: Some(results),
        ..
    }) = fcm_client.send(builder.finalize()).await
    {
        let rejected: Vec<String> = results
            .iter()
            .filter_map(|result| result.error.and_then(|_| result.registration_id.clone()))
            .collect();
        info!("Rejected: {:?}", &rejected);
        Ok(HttpResponse::Ok().json(&PushGatewayResponse { rejected }))
    } else {
        Err(MatrixError {
            error: String::from("Invalid response from upstream push service"),
            errcode: ErrCode::MUnknown,
        })
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    let config = Settings::load().expect("Config file (config.toml) is not present");
    info!("Now listening to port {}", config.server_port);
    let app_config = web::Data::new(config.clone());
    let fcm_client = web::Data::new(fcm::Client::new());

    HttpServer::new(move || {
        App::new()
            .app_data(app_config.clone())
            .app_data(fcm_client.clone())
            .app_data(web::JsonConfig::default().error_handler(json_error_handler))
            .route(
                "/_matrix/push/v1/notify",
                web::post().to(process_notification),
            )
    })
    .bind(("127.0.0.1", config.server_port))?
    .run()
    .await
}

fn json_error_handler(err: JsonPayloadError, _: &actix_web::HttpRequest) -> actix_web::Error {
    if let JsonPayloadError::Deserialize(deserialize_err) = &err {
        if deserialize_err.classify() == serde_json::error::Category::Data {
            return MatrixError {
                error: deserialize_err.to_string(),
                errcode: ErrCode::MMissingParam,
            }
            .into();
        }
    }
    MatrixError {
        error: err.to_string(),
        errcode: ErrCode::MBadJson,
    }
    .into()
}
