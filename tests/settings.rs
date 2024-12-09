//! Tests for the settings module.

#![allow(clippy::unwrap_used)]

use matrix_hedwig::settings;

#[test]
fn load_settings() {
	settings::Settings::load("config.sample.yaml").unwrap();
	settings::Settings::load("tests/config-bad.yaml").unwrap_err();
}
