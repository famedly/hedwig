extern crate vergen;

use vergen::{vergen, Config};

fn main() {
	if let Err(e) = vergen(Config::default()) {
		println!("cargo:warning=Vergen failed to generate additional build metadata: {}", e);
	}
}
