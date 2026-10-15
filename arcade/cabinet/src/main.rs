//! The shareware arcade cabinet — all five carts wired into the full console.
//!
//! Launch with `cargo run -p cabinet` from inside `shareware/arcade/`.

use arcade_cart::{ConsoleConfig, Game};

fn main() {
    let games: Vec<Box<dyn Game>> = vec![
        Box::new(skifree::SkiFree::new()),
        Box::new(pong::Pong::new()),
        Box::new(breakout::Breakout::new()),
        Box::new(snake::SnakeGame::new()),
        Box::new(etch::Sketch::new()),
    ];

    if let Err(err) = arcade_cart::runner::run_console(games, ConsoleConfig::default()) {
        eprintln!("cabinet: failed to start window: {err}");
        std::process::exit(1);
    }
}
