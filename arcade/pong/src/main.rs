fn main() {
    if let Err(err) = arcade_cart::runner::run_single(pong::Pong::new()) {
        eprintln!("pong: failed to start window: {err}");
        std::process::exit(1);
    }
}
