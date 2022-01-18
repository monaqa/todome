pub mod language_server;
pub mod structure;
pub mod subcmd;

pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
