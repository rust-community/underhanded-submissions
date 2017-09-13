#[macro_use]
extern crate simple_payment_service;

use simple_payment_service::server;

fn main() {
    server::create_server("127.0.0.1:6767").unwrap();
    println!("Running server!");
}
