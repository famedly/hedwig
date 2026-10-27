//! Tests for the settings module.

#![allow(clippy::unwrap_used)]

use std::{fs, io::Write};

use matrix_hedwig::settings;
use tempfile::Builder as TempFileBuilder;

#[test]
fn load_settings() {
	settings::Settings::load("config.sample.yaml").unwrap();
	settings::Settings::load("tests/config-bad.yaml").unwrap_err();
	settings::Settings::load("tests/config-bad-apns.yaml").unwrap_err();
}

#[test]
/// APNS configuration should be optional
fn load_settings_without_apns_credentials() {
	let config = fs::read_to_string("config.sample.yaml")
		.unwrap()
		.lines()
		.filter(|line| {
			let line = line.trim_start();
			!["apns_key_file_path:", "apns_team_id:", "apns_key_id:"]
				.iter()
				.any(|setting| line.starts_with(setting))
		})
		.collect::<Vec<_>>()
		.join("\n");
	let mut config_file = TempFileBuilder::new().suffix(".yaml").tempfile().unwrap();
	config_file.write_all(config.as_bytes()).unwrap();

	let settings = temp_env::with_vars_unset(
		[
			"PUSHGW__HEDWIG__APNS_KEY_FILE_PATH",
			"PUSHGW__HEDWIG__APNS_TEAM_ID",
			"PUSHGW__HEDWIG__APNS_KEY_ID",
			"PUSHGW__HEDWIG__APNS_SANDBOX",
		],
		|| settings::Settings::load(config_file.path().to_str().unwrap()),
	);

	let settings = settings.unwrap();
	assert_eq!(settings.hedwig.apns_key_file_path, None);
	assert_eq!(settings.hedwig.apns_team_id, None);
	assert_eq!(settings.hedwig.apns_key_id, None);
	assert!(settings.hedwig.apns_sandbox);
}
