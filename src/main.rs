use actix_web::{web, App, HttpResponse, HttpServer};
mod models;
mod settings;
use actix_web::error::JsonPayloadError;
use fcm::FcmResponse;
use models::{ErrCode, MatrixError, PushGatewayResponse, PushNotification};
use settings::Settings;

async fn process_notification(
    push_notification: web::Json<PushNotification>,
    config: web::Data<Settings>,
    fcm_client: web::Data<fcm::Client>,
) -> Result<HttpResponse, MatrixError> {
    // Collect the pushkeys and check if the app_id of each device matches
    let registration_ids: Vec<String> = push_notification
        .notification
        .devices
        .iter()
        .filter_map(|device| {
            if device.app_id == config.app_id
                || device.app_id == format!("{}.data_message", config.app_id)
            {
                Some(device.pushkey.clone())
            } else {
                None
            }
        })
        .collect();

    if registration_ids.is_empty() {
        return Err(MatrixError {
            error: String::from("No registration IDs with the matching app ID provided"),
            errcode: ErrCode::MBadJson,
        });
    }

    let first_device = if let Some(device) = push_notification.notification.devices.first() {
        device
    } else {
        return Err(MatrixError {
            error: String::from("No devices were provided"),
            errcode: ErrCode::MBadJson,
        });
    };

    // Weither the push gateway should send only a data message - we have a specific app_id suffix for this
    let data_message_mode = first_device.app_id == format!("{}.data_message", config.app_id);

    // Some notifications may just inform the device that there are no more unread rooms
    let is_clearing_notification = push_notification.notification.event_id.is_none();

    // String representation of the unread counter with "0" as default
    let unread_string = match &push_notification.notification.counts {
        Some(counts) => match counts.unread {
            Some(unread) => format!("{}", unread),
            None => String::from("0"),
        },
        None => String::from("0"),
    };

    // Get the MessageBuilder - either we need to send the notification to one or to multiple devices
    let mut builder = if registration_ids.len() == 1 {
        fcm::MessageBuilder::new(&config.fcm_admin_key, registration_ids.first().unwrap())
    } else {
        fcm::MessageBuilder::new_multi(&config.fcm_admin_key, &registration_ids)
    };
    // Set the data for fcm here
    builder.collapse_key(&config.fcm_collapse_key);
    builder
        .data(&push_notification.notification)
        .map_err(|e| MatrixError {
            errcode: ErrCode::MBadJson,
            error: e.to_string(),
        })?;
    // builder.data(&map);

    // Set additional keys for the notification message

    if !data_message_mode && !is_clearing_notification {
        let mut notification = fcm::NotificationBuilder::new();
        notification
            .title(&config.fcm_notification_title)
            .click_action(&config.fcm_notification_click_action)
            .body(&config.fcm_notification_body)
            .badge(&unread_string)
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

        Ok(HttpResponse::Ok().json(PushGatewayResponse { rejected }))
    } else {
        Err(MatrixError {
            error: String::from("Invalid response from upstream push service"),
            errcode: ErrCode::MUnknown,
        })
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Settings::load().expect("Config file (config.toml) is not present");
    println!("Now listening to port {}", config.server_port);
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
