//! Add build information.
#![allow(clippy::expect_used)]

use vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder, RustcBuilder};

fn main() -> anyhow::Result<()> {
	Emitter::default()
		.add_instructions(&BuildBuilder::all_build()?)?
		.add_instructions(&CargoBuilder::all_cargo()?)?
		.add_instructions(&RustcBuilder::all_rustc()?)?
		.add_instructions(&GitclBuilder::all_git()?)?
		.emit()
}
