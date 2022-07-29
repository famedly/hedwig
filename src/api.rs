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

use std::{
	net::SocketAddr,
	sync::Arc,
	time::{Duration, Instant},
};

use axum::{
	routing::{get, post},
	Extension, Json, Router,
};
use opentelemetry::KeyValue;
use tokio::sync::{Mutex, RwLock};
use tracing::debug;

use crate::{
	fcm::FcmSender,
	jitter::Jitter,
	models::{self, PushGatewayResponse},
	pusher,
	settings::Settings,
};

/// Endpoint for matrix push
pub async fn matrix_push(
	notification: models::Notification,
	Extension(jitter): Extension<Arc<RwLock<Jitter>>>,
	Extension(fcm_sender): Extension<Arc<Mutex<Box<dyn FcmSender + Send + Sync>>>>,
	Extension(settings): Extension<Arc<Settings>>,
	Extension(counters): Extension<Arc<models::Metrics>>,
) -> Json<PushGatewayResponse> {
	let mut rejected: Vec<String> = Vec::new();

	let start = Instant::now();

	// TODO: as it stands, this way of implementing jitter will result in messages
	// arriving out of order especially on lower traffic hedwig instances!
	let jitter_roll = jitter.read().await.get_jitter_delay();
	counters.jitter.record(jitter_roll.as_secs_f64(), &[]);
	tokio::time::sleep(jitter_roll).await;

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
					debug!("A push failed: {:?}", e);
					counters
						.failed_pushes
						.add(1, &[KeyValue::new("device_type", device_type.clone())]);
					rejected.push(dev.pushkey.clone());
					break;
				}
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

	// If we pushed anything successfully it counts towards the jitter frequency
	if rejected.len() < notification.devices.len() {
		jitter.write().await.push_successful_jitter(start);
	}

	Json(PushGatewayResponse { rejected })
}

/// Version of the crate
const VERSION: &str = match option_env!("VERGEN_GIT_SEMVER") {
	Some(version) => version,
	None => env!("CARGO_PKG_VERSION"),
};

/// Sets up and runs the server
pub async fn run_server(settings: Settings, fcm_sender: Box<dyn FcmSender + Send + Sync>) {
	let jitter = Jitter::new(Duration::from_secs_f64(settings.hedwig.max_jitter_delay));
	let addr: SocketAddr = (settings.server.bind_address, settings.server.port).into();

	let metrics_middleware =
		axum_opentelemetry_middleware::RecorderMiddlewareBuilder::new("Hedwig");
	let metrics = models::Metrics::new(&metrics_middleware.meter);

	let app = Router::new()
		.route("/metrics", get(axum_opentelemetry_middleware::metrics_endpoint))
		.route("/_matrix/push/v1/notify", post(matrix_push))
		.route("/health", get(|| async { "" }))
		.route("/version", get(|| async { VERSION }))
		.layer(Extension(Arc::new(RwLock::new(jitter))))
		.layer(Extension(Arc::new(Mutex::new(fcm_sender))))
		.layer(Extension(Arc::new(settings)))
		.layer(Extension(Arc::new(metrics)))
		.layer(metrics_middleware.build());

	// If this fails the address+port can't be listened to right now
	#[allow(clippy::expect_used)]
	axum::Server::bind(&addr)
		.serve(app.into_make_service())
		.await
		.expect("Failed to bind address!");
}
