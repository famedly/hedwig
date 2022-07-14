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
mod error;
mod fcm;
mod jitter;
mod models;
mod pusher;
mod settings;

use std::str::FromStr;

use color_eyre::{eyre::WrapErr, Report};
use tracing::info;
use tracing_subscriber::FmtSubscriber;

use crate::fcm::FcmSenderImpl;

#[tokio::main]
async fn main() -> Result<(), Report> {
	// Complete failure if config file is missing
	#[allow(clippy::expect_used)]
	let settings = settings::Settings::load()
		.expect("Failed to load config, be sure to put it in config.toml");

	let subscriber = FmtSubscriber::builder()
		.with_max_level(
			tracing::Level::from_str(&settings.log.level)
				.wrap_err("Initializing logging failed")?,
		)
		.finish();

	tracing::subscriber::set_global_default(subscriber)
		.wrap_err("Setting up default subscriber")?;

	info!("{:?}", settings);

	// If fcm authentication fails there's no use continuing
	#[allow(clippy::expect_used)]
	let fcm_auth = FcmSenderImpl::new(&settings.hedwig.fcm_service_account_token_path)
		.expect("Failed to authenticate with fcm");

	api::run_server(settings, Box::new(fcm_auth)).await;

	Ok(())
}
