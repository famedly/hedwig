//! Api server implementation for a matrix push gateway

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

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
	extract::{DefaultBodyLimit, FromRef, State},
	routing::{get, post},
	Json, Router,
};
use axum_opentelemetry_middleware::RecorderMiddleware;
use color_eyre::{eyre::WrapErr, Report};
use opentelemetry::KeyValue;
use tokio::sync::Mutex;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::{debug, info};

use crate::{
	fcm::FcmSender,
	models::{Metrics, Notification, PushGatewayResponse},
	pusher,
	settings::Settings,
};

/// Endpoint for matrix push
pub async fn matrix_push(
	State(fcm_sender): State<Arc<Mutex<Box<dyn FcmSender + Send + Sync>>>>,
	State(settings): State<Arc<Settings>>,
	State(counters): State<Arc<Metrics>>,
	notification: Notification,
) -> Json<PushGatewayResponse> {
	let mut rejected: Vec<String> = Vec::new();

	debug!("Got notification to be pushed to {} devices.", notification.devices.len());
	for dev in &notification.devices {
		let device_type = if dev.app_id.ends_with(".data_message") {
			"AndroidLegacy".to_owned()
		} else {
			format!("{:?}", dev.data_message_type())
		};

		let mut retry_time = Duration::from_millis(250);
		let mut attempt = 0;
		loop {
			if let Err(e) =
				pusher::push_notification(&notification, dev, &fcm_sender, &settings).await
			{
				attempt += 1;
				if attempt > settings.hedwig.fcm_push_max_retries {
					info!(
						"A push failed (device type: {}), even after retrying: {}",
						device_type, e
					);
					counters
						.failed_pushes
						.add(1, &[KeyValue::new("device_type", device_type.clone())]);
					rejected.push(dev.pushkey.clone());
					break;
				}
				debug!("A push failed, retrying in a bit. (Error: {})", e);

				tokio::time::sleep(retry_time).await;
				retry_time *= 2;
			} else {
				counters
					.successful_pushes
					.add(1, &[KeyValue::new("device_type", device_type.clone())]);
				break;
			}
		}
	}

	if rejected.len() < notification.devices.len() {
		counters.notifications.add(
			1,
			[notification.r#type.map(|r#type| KeyValue::new("notification_type", r#type))]
				.into_iter()
				.flatten()
				.collect::<Vec<_>>()
				.as_slice(),
		);
	}

	counters.devices.add(notification.devices.len() as u64, &[]);

	Json(PushGatewayResponse { rejected })
}

/// Version of the crate
const VERSION: &str = match option_env!("VERGEN_GIT_SEMVER") {
	Some(version) => version,
	None => env!("CARGO_PKG_VERSION"),
};

/// Struct holding shared state, settings and interfaces for the Hedwig router
#[derive(Clone, FromRef, Debug)]
pub struct AppState {
	/// [FcmSender] for communication with Firebase
	/// Usually [crate::fcm::FcmSenderImpl]
	fcm_sender: Arc<Mutex<Box<dyn FcmSender + Send + Sync>>>,
	/// Hedwig [Settings]
	settings: Arc<Settings>,
	/// Prometheus [Metrics]
	counters: Arc<Metrics>,
}

impl AppState {
	/// Bundle state into [AppState]
	#[must_use]
	pub fn new(
		fcm_sender: Box<dyn FcmSender + Send + Sync>,
		settings: Settings,
		counters: Metrics,
	) -> Self {
		AppState {
			fcm_sender: Arc::new(Mutex::new(fcm_sender)),
			settings: Arc::new(settings),
			counters: Arc::new(counters),
		}
	}
}

/// Create main Hedwig router.
///
/// # Errors
///
/// This function will return [std::num::TryFromIntError] if the body limit is
/// larger than the target architectures usize range(This should never happen)
pub fn create_router(
	app_state: AppState,
	metrics_middleware: RecorderMiddleware,
) -> Result<Router, Report> {
	let settings = app_state.settings.clone();

	let usized_limit: usize = settings.hedwig.notification_request_body_size_limit.try_into()?;
	let notification_body_limit = DefaultBodyLimit::max(usized_limit);

	let router = Router::new()
		.route("/metrics", get(axum_opentelemetry_middleware::metrics_endpoint))
		.route("/_matrix/push/v1/notify", post(matrix_push).layer(notification_body_limit))
		.route("/health", get(|| async { "" }))
		.route("/version", get(|| async { VERSION }))
		.with_state(app_state)
		// Also takes trailing slash to avoid potential incompabilities
		.layer(NormalizePathLayer::trim_trailing_slash())
		.layer(metrics_middleware);

	Ok(router)
}

/// Sets up and runs the server
pub async fn run_server(
	settings: Settings,
	fcm_sender: Box<dyn FcmSender + Send + Sync>,
) -> Result<(), Report> {
	let addr: SocketAddr = (settings.server.bind_address, settings.server.port).into();

	let metrics_middleware =
		axum_opentelemetry_middleware::RecorderMiddlewareBuilder::new("Hedwig");
	let metrics = Metrics::new(&metrics_middleware.meter);

	let app_state = AppState::new(fcm_sender, settings, metrics);

	let router = create_router(app_state, metrics_middleware.build())?;

	let listener =
		tokio::net::TcpListener::bind(&addr).await.wrap_err("Failed to bind to address")?;

	axum::serve(listener, router).await.wrap_err("Failed to start api server")
}
