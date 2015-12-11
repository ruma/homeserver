extern crate serde_codegen;
extern crate syntex;

use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src = Path::new("src/error.in.rs");
    let dst = Path::new(&out_dir).join("error.rs");

    let mut registry = syntex::Registry::new();

    serde_codegen::register(&mut registry);
    registry.expand("", &src, &dst).unwrap();
}
