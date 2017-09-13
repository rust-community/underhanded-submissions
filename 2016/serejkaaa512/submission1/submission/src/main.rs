#[macro_use]
extern crate underhanded;

use underhanded::server;

fn main() {
    server::create_server("127.0.0.1:6767").unwrap();
    println!("Running server!");
}
