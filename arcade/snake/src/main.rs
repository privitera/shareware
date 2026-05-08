fn main() {
    if let Err(err) = arcade_cart::runner::run_single(snake::SnakeGame::new()) {
        eprintln!("snake: failed to start window: {err}");
        std::process::exit(1);
    }
}
