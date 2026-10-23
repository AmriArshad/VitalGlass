fn main() {
    match vitalglass_core::config::ClientConfig::load() {
        Ok(_) => eprintln!("Client config loaded."),
        Err(e) => eprintln!("{e}"),
    }
}
