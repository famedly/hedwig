//! Tests for the settings module.

#![allow(clippy::unwrap_used)]

use std::{fs, path::PathBuf};

use matrix_hedwig::settings;
use tempfile::Builder as TempFileBuilder;

/// Writes `config` to a temporary yaml file and loads it with the APNS
/// environment overrides unset, so the ambient environment can't influence the
/// result.
fn load_config(config: &str) -> settings::Settings {
	let config_file = TempFileBuilder::new().suffix(".yaml").tempfile().unwrap();
	fs::write(config_file.path(), config).unwrap();

	temp_env::with_vars_unset(
		[
			"PUSHGW__HEDWIG__APNS_KEY_FILE_PATH",
			"PUSHGW__HEDWIG__APNS_TEAM_ID",
			"PUSHGW__HEDWIG__APNS_KEY_ID",
			"PUSHGW__HEDWIG__APNS_SANDBOX",
		],
		|| settings::Settings::load(config_file.path().to_str().unwrap()),
	)
	.unwrap()
}

/// Loads `config.sample.yaml` with the given settings removed, so configs
/// missing optional values can be exercised without keeping near-duplicates of
/// the sample config around.
fn load_sample_without(dropped_settings: &[&str]) -> settings::Settings {
	let prefixes: Vec<String> =
		dropped_settings.iter().map(|setting| format!("{setting}:")).collect();
	let config = fs::read_to_string("config.sample.yaml")
		.unwrap()
		.lines()
		.filter(|line| {
			let line = line.trim_start();
			!prefixes.iter().any(|prefix| line.starts_with(prefix))
		})
		.collect::<Vec<_>>()
		.join("\n");

	load_config(&config)
}

#[test]
/// The sample config loads, garbage doesn't.
fn load_settings() {
	settings::Settings::load("config.sample.yaml").unwrap();
	settings::Settings::load("tests/config-bad.yaml").unwrap_err();
}

#[test]
/// With none of the APNS settings configured there is nothing to authenticate
/// with, so direct APNS stays switched off instead of erroring.
fn apns_credentials_are_absent_without_any_apns_settings() {
	let settings = load_sample_without(&["apns_key_file_path", "apns_team_id", "apns_key_id"]);

	assert_eq!(settings.hedwig.apns_key_file_path, None);
	assert_eq!(settings.hedwig.apns_team_id, None);
	assert_eq!(settings.hedwig.apns_key_id, None);
	assert!(settings.hedwig.apns_sandbox);
	assert_eq!(settings.hedwig.apns_credentials().unwrap(), None);
}

#[test]
/// A fully configured APNS section yields the credentials.
fn apns_credentials_are_returned_when_fully_configured() {
	let settings = load_sample_without(&[]);

	let credentials = settings.hedwig.apns_credentials().unwrap().unwrap();
	assert_eq!(credentials.key_file_path, PathBuf::from("path/to/apns_key.p8"));
	assert_eq!(credentials.team_id, "YOUR_TEAM_ID");
	assert_eq!(credentials.key_id, "YOUR_KEY_ID");
}

#[test]
/// The three APNS settings are only optional together: every partial
/// combination is rejected rather than silently disabling direct APNS or
/// ignoring leftover credentials.
fn partial_apns_configuration_is_rejected() {
	for dropped in [
		vec!["apns_key_file_path"],
		vec!["apns_team_id"],
		vec!["apns_key_id"],
		vec!["apns_key_file_path", "apns_team_id"],
		vec!["apns_key_file_path", "apns_key_id"],
		vec!["apns_team_id", "apns_key_id"],
	] {
		let settings = load_sample_without(&dropped);

		let error = settings.hedwig.apns_credentials().unwrap_err();
		assert_eq!(error.missing, dropped, "wrong settings reported as missing");
	}
}

#[test]
/// Deployment tooling that substitutes an unset variable
/// (`APNS_TEAM_ID=${APNS_TEAM_ID}`) hands hedwig empty strings rather than
/// nothing at all. They mean "unset", so an FCM-only deployment still starts.
fn empty_apns_settings_count_as_unset() {
	let mut settings = load_sample_without(&["apns_key_file_path", "apns_team_id", "apns_key_id"]);
	settings.hedwig.apns_key_file_path = Some(PathBuf::new());
	settings.hedwig.apns_team_id = Some(String::new());
	settings.hedwig.apns_key_id = Some(String::new());

	assert_eq!(settings.hedwig.apns_credentials().unwrap(), None);
}

#[test]
/// An empty setting next to configured ones is still a partial configuration,
/// and is reported as missing rather than as set-but-blank.
fn empty_apns_setting_is_reported_as_missing() {
	let mut settings = load_sample_without(&[]);
	settings.hedwig.apns_team_id = Some(String::new());

	let error = settings.hedwig.apns_credentials().unwrap_err();
	assert_eq!(error.missing, vec!["apns_team_id"]);
}
