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

use std::{collections::HashMap, fs::File, io, net::SocketAddr, str::FromStr, sync::Arc};

use axum::{
	http::StatusCode,
	routing::{get, get_service, post},
	AddExtensionLayer, Router,
};
use color_eyre::{eyre::WrapErr, Report};
use matrix_hedwig::{
	handlers::process_notification,
	metrics::{DeviceCounter, LastSuccessfulCollector, NotificationCounter},
	settings,
};
use prometheus::{opts, IntCounterVec, Registry};
use settings::{Pusher, Settings};
use tower_http::services::ServeFile;
use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;

const VERSION: &str = match option_env!("VERGEN_GIT_SEMVER") {
	Some(version) => version,
	None => env!("CARGO_PKG_VERSION"),
};

#[tokio::main]
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
	//	let prometheus = PrometheusMetrics::new_with_registry(registry, "api",
	// Some("/metrics"), None) 		.map_err(|e| eyre!("Initializing prometheus: {}",
	// e))?;

	let apns_clients = config
		.pushers
		.iter()
		.filter_map(|(k, v)| {
			if let Pusher::Apns(config) = v {
				Some((|| {
					let mut private_key =
						File::open(&config.key_file).wrap_err("Could not read apns key file")?;
					let client = a2::Client::token(
						&mut private_key,
						&config.key_id,
						&config.team_id,
						config.endpoint.to_owned().into(),
					)
					.wrap_err("Could not create apns client object")?;
					Ok((k.to_owned(), client))
				})())
			} else {
				None
			}
		})
		.collect::<Result<HashMap<String, a2::Client>, Report>>()?;

	info!("Now listening on {}:{}", config.server.bind_address, config.server.port);

	// build our application with a route
	let app = Router::new()
		.route("/_matrix/push/v1/notify", post(process_notification))
		.route("/health", get(|| async { "" }))
		.route("/version", get(|| async { VERSION }))
		.route(
			"/favicon.ico",
			get_service(ServeFile::new("./static/favicon.ico")).handle_error(
				|error: io::Error| async move {
					(
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("Unhandled internal error: {}", error),
					)
				},
			),
		)
		.layer(AddExtensionLayer::new(config.clone()))
		.layer(AddExtensionLayer::new(Arc::new(fcm::Client::new())))
		.layer(AddExtensionLayer::new(Arc::new(apns_clients)))
		.layer(AddExtensionLayer::new(notification_counter))
		.layer(AddExtensionLayer::new(device_counter))
		.layer(AddExtensionLayer::new(last_successful_gauge));

	// TODO: prometheus

	let addr: SocketAddr = format!("{}:{}", &config.server.bind_address, &config.server.port)
		.parse()
		.wrap_err("Failed to construct the bind address")?;
	debug!("listening on {}", addr);
	axum::Server::bind(&addr)
		.serve(app.into_make_service())
		.await
		.wrap_err("HTTP Server failed")?;
	Ok(())
}
