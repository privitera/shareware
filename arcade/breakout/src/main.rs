fn main() {
    if let Err(err) = arcade_cart::runner::run_single(breakout::Breakout::new()) {
        eprintln!("breakout: failed to start window: {err}");
        std::process::exit(1);
    }
}
