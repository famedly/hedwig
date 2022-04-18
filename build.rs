//! Add build information.
#![allow(clippy::expect_used)]

use vergen::{vergen, Config, SemverKind, ShaKind};

fn main() {
	let mut cfg = Config::default();
	*cfg.git_mut().semver_kind_mut() = SemverKind::Lightweight;
	*cfg.git_mut().sha_kind_mut() = ShaKind::Normal;
	vergen(cfg).expect("Unable to generate cargo keys!");
}
