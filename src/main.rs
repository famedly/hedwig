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

use std::str::FromStr;

use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use actix_web_prom::PrometheusMetrics;
use color_eyre::{
	eyre::{eyre, WrapErr},
	Report,
};
use matrix_hedwig::{
	handlers::{json_error_handler, process_notification},
	metrics::{DeviceCounter, LastSuccessfulCollector, NotificationCounter},
	settings,
};
use prometheus::{opts, IntCounterVec, Registry};
use settings::Settings;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> Result<(), Report> {
	let config = Settings::load().wrap_err("Couldn't load the config")?;

	let subscriber = FmtSubscriber::builder()
		.with_max_level(
			tracing::Level::from_str(&config.log.level).wrap_err("Initializing logging failed")?,
		)
		.finish();

	tracing::subscriber::set_global_default(subscriber)
		.wrap_err("Setting up default subscriber")?;

	let notification_counter = NotificationCounter(
		IntCounterVec::new(
			opts!("notifications_total", "Notification statistics").namespace("hedwig"),
			&["type", "status"],
		)
		.wrap_err("Setting up the Prometheus notification counter")?,
	);
	let device_counter = DeviceCounter(
		IntCounterVec::new(
			opts!("devices_total", "Notification statistics").namespace("hedwig"),
			&[],
		)
		.wrap_err("Setting up the Prometheus device counter")?,
	);

	let last_successful_gauge = LastSuccessfulCollector::new(
		"last_notification_seconds",
		"Seconds since the last successful notification was sent",
	)
	.wrap_err("Creating a last succesful gauge")?;

	let registry = Registry::new();
	registry
		.register(Box::new(notification_counter.0.clone()))
		.wrap_err("Registering a notification counter")?;
	registry
		.register(Box::new(device_counter.0.clone()))
		.wrap_err("Registering a device counter")?;
	registry
		.register(Box::new(last_successful_gauge.clone()))
		.wrap_err("Registering a last succesful gauge")?;
	let prometheus = PrometheusMetrics::new_with_registry(registry, "api", Some("/metrics"), None)
		.map_err(|e| eyre!("Initializing prometheus: {}", e))?;

	info!("Now listening on {}:{}", config.server.bind_address, config.server.port);
	let app_config = web::Data::new(config.clone());
	let fcm_client = web::Data::new(fcm::Client::new());

	HttpServer::new(move || {
		App::new()
			.app_data(app_config.clone())
			.app_data(fcm_client.clone())
			.app_data(web::Data::new(notification_counter.clone()))
			.app_data(web::Data::new(device_counter.clone()))
			.app_data(web::Data::new(last_successful_gauge.clone()))
			.app_data(web::JsonConfig::default().error_handler(json_error_handler))
			.wrap(Logger::default())
			.wrap(prometheus.clone())
			.service(web::resource("/favicon.ico").to(|| {
				HttpResponse::Ok()
					.content_type("image/png")
					.body(&include_bytes!("../static/favicon.ico")[..])
			}))
			.service(web::resource("/health").to(|| actix_web::HttpResponse::Ok().finish()))
			.service(
				web::resource("/version")
					.to(|| actix_web::HttpResponse::Ok().body(env!("VERGEN_GIT_SEMVER"))),
			)
			.route("/_matrix/push/v1/notify", web::post().to(process_notification))
	})
	.bind((config.server.bind_address, config.server.port))?
	.run()
	.await
	.wrap_err("Initializing HTTP server")?;
	Ok(())
}
