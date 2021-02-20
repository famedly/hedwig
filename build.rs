extern crate vergen;

use vergen::{gen, ConstantsFlags};

fn main() {
    gen(ConstantsFlags::all()).expect("Unable to generate cargo keys!");
}
