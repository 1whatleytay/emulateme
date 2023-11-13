use std::fs;
use std::io::Result;

fn main() -> Result<()> {
    fs::create_dir_all("../src").expect("Out dir must be created for prost_build.");

    prost_build::Config::default()
        .out_dir("../src")
        .compile_protos(&["../src/messages.proto"], &["../src/"])
}
