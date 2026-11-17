//! A matrix push gateway implementation using FCM with support for both Android
//! and iOS notifications

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

mod api;
mod apns;
mod error;
mod fcm;
mod health;
mod metrics;
mod models;
mod pusher;
mod settings;

use color_eyre::{Report, eyre::WrapErr};
use tracing::info;

use crate::{apns::APNSSenderImpl, fcm::FcmSenderImpl};

#[tokio::main]
// Need to be able to print errors before the logger is up
#[allow(clippy::print_stderr)]
async fn main() -> Result<(), Report> {
	match std::env::args().nth(1).as_deref() {
		// Also proves at image build time that the binary runs on the image's base.
		Some("--version") => {
			#[allow(clippy::print_stdout)]
			{
				println!("{}", env!("CARGO_PKG_VERSION"));
			}
			return Ok(());
		}
		// The container health check: distroless images have no curl.
		Some("health") => {
			let settings = settings::Settings::load(settings::Settings::CONFIG_FILENAME)?;
			return health::probe(&settings.server).wrap_err("health check failed");
		}
		_ => {}
	}

	// Complete failure if config file is missing
	let settings = settings::Settings::load(settings::Settings::CONFIG_FILENAME)?;

	rust_telemetry::init_otel(&settings.telemetry, "Hedwig", env!("CARGO_PKG_VERSION"), "Hedwig")?;

	info!("Launching with settings: {:?}", settings);

	let fcm_auth = FcmSenderImpl::new(&settings.hedwig.fcm_credentials_file_path)
		.await
		.wrap_err("Fcm setup failed")?;

	let apns_auth = settings
		.hedwig
		.apns_credentials()?
		.map(|credentials| {
			APNSSenderImpl::new(
				credentials.key_file_path,
				credentials.team_id,
				credentials.key_id,
				settings.hedwig.apns_sandbox,
			)
		})
		.transpose()
		.wrap_err("APNS authentication failed")?;
	info!("Starting server");
	api::run_server(settings, Box::new(fcm_auth), apns_auth).await?;

	Ok(())
}
