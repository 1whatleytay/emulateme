mod server;
mod messages;
mod delimiter;

use std::{env, fs};
use emulateme::rom::{parse_rom};
use crate::server::run_server;

#[tokio::main]
async fn main() {
    let arguments: Vec<String> = env::args().collect();
    let path = arguments.get(1)
        .expect("Requires one argument, a path to a valid NES ROM.");

    let bytes = fs::read(path)
        .unwrap_or_else(|_| panic!("Cannot find ROM at path {path}"));

    let (_, rom) = parse_rom(&bytes)
        .unwrap_or_else(|_| panic!("Failed to parse ROM contents at path {path}"));

    run_server(&rom, "127.0.0.1:9013").await.unwrap()
}
