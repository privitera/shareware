fn main() {
    if let Err(err) = arcade_cart::runner::run_single(skifree::SkiFree::new()) {
        eprintln!("skifree: failed to start window: {err}");
        std::process::exit(1);
    }
}
