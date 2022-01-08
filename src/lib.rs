// pub mod format;
pub mod language_server;
pub mod structure;

pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
