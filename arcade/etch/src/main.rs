fn main() {
    if let Err(err) = arcade_cart::runner::run_single(etch::Sketch::new()) {
        eprintln!("etch: failed to start window: {err}");
        std::process::exit(1);
    }
}
