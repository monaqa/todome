pub mod capabilities;
pub mod language_server;
pub mod parser;

pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
