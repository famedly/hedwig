//! Implements the logic for collecting and exposing Prometheus metrics

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
	sync::Arc,
	task::{Context, Poll},
	time::Instant,
};

use axum::{
	body::Body,
	extract::{MatchedPath, State},
	http::Request,
	response::Response,
};
use futures::future::BoxFuture;
use opentelemetry::KeyValue;
use tower::Service;

use crate::models::Metrics;

/// exposes Prometheus metrics
pub async fn metrics_handler(State(registry): State<Arc<prometheus::Registry>>) -> String {
	use prometheus::{Encoder, TextEncoder};
	let encoder = TextEncoder::new();
	let metric_families = registry.gather();
	let mut buffer = Vec::new();
	encoder.encode(&metric_families, &mut buffer).unwrap_or_default();
	String::from_utf8(buffer).unwrap_or_default()
}

/// Middleware for recording HTTP request metrics
#[derive(Debug, Clone)]
pub struct HttpMetricsMiddleware {
	/// Histogram tracking the duration of each HTTP request
	http_requests_duration_seconds: opentelemetry::metrics::Histogram<f64>,
	/// Counter tracking the total number of HTTP requests
	http_requests_total: opentelemetry::metrics::Counter<u64>,
}

impl HttpMetricsMiddleware {
	/// Create a new HTTP metrics middleware
	#[must_use]
	pub fn new(metrics: Arc<Metrics>) -> Self {
		Self {
			http_requests_duration_seconds: metrics.http_requests_duration_seconds.clone(),
			http_requests_total: metrics.http_requests_total.clone(),
		}
	}
}

impl<S> tower::Layer<S> for HttpMetricsMiddleware {
	type Service = HttpMetricsService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		HttpMetricsService {
			inner,
			http_requests_duration_seconds: self.http_requests_duration_seconds.clone(),
			http_requests_total: self.http_requests_total.clone(),
		}
	}
}

/// Service implementation for HTTP metrics middleware
#[derive(Debug, Clone)]
pub struct HttpMetricsService<S> {
	/// Inner service
	inner: S,
	/// Histogram tracking the duration of each HTTP request
	http_requests_duration_seconds: opentelemetry::metrics::Histogram<f64>,
	/// Counter tracking the total number of HTTP requests
	http_requests_total: opentelemetry::metrics::Counter<u64>,
}

impl<S> Service<Request<Body>> for HttpMetricsService<S>
where
	S: Service<Request<Body>, Response = Response> + Send + 'static,
	S::Future: Send + 'static,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		let method = req.method().as_str().to_owned();
		let matched_path =
			req.extensions().get::<MatchedPath>().map(|path| path.as_str().to_owned());
		let future = self.inner.call(req);
		let http_requests_duration_seconds = self.http_requests_duration_seconds.clone();
		let http_requests_total = self.http_requests_total.clone();

		Box::pin(async move {
			let Some(path) = matched_path else {
				return future.await;
			};

			let start = Instant::now();

			let resp = future.await?;

			let duration = start.elapsed();
			let status = resp.status().as_str().to_owned();

			let attributes = [
				KeyValue::new("endpoint", path),
				KeyValue::new("method", method),
				KeyValue::new("status", status),
			];

			http_requests_duration_seconds.record(duration.as_secs_f64(), &attributes);
			http_requests_total.add(1, &attributes);

			Ok(resp)
		})
	}
}
