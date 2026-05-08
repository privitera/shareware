# arcade

A small console-and-cartridge architecture for retro mini-games, written in
Rust on top of [`egui`](https://github.com/emilk/egui) / `eframe`. Five carts
ship out of the box — all bundled into a single home screen with a CRT power-off
animation when you exit.

```
+----------------+   +-----------------------------+
|  arcade-cart   |   |  skifree   pong   breakout  |
|   (the spec)   |←──|   snake    etch             |
|                |   +-----------------------------+
|  Game trait    |          ↑          ↑
|  Input         |          │          │
|  Rng           |          │     +────┴────+
|  Console       |          └─────│ cabinet │  (the bundled bin)
|  ConsoleConfig |                +─────────+
|  runner::*     |                     ↑
+----------------+                     │
                                  cargo run -p cabinet
```

## Quick start

```bash
cd shareware/arcade

# Run the full console — all five carts, home screen, CRT shutdown
cargo run -p cabinet

# Or run a single cart standalone
cargo run -p skifree
cargo run -p pong
cargo run -p breakout
cargo run -p snake
cargo run -p etch
```

## Keyboard controls

The standalone runner maps keys to the rotational input model:

| Key            | Action                                    |
|----------------|-------------------------------------------|
| `A` / `D`      | Decrement / increment `rotation_left`     |
| `←` / `→`      | Decrement / increment `rotation_right`    |
| `rotation`     | Sum of left + right (used by 1P games)    |
| `Space`        | Action button                             |
| `Esc`          | Return to menu (or exit single-game)      |
| `Q`            | Trigger CRT shutdown + exit (cabinet)     |

In the cabinet, the home screen uses `rotation` to scroll the menu and `Space`
to start a game.

## The carts

| Cart       | Source              | Notes                                 |
|------------|---------------------|---------------------------------------|
| `skifree`  | Chris Pirih, 1991   | Sprites from `basicallydan/skifree.js` (MIT). See [`skifree/NOTICE`](./skifree/NOTICE). |
| `pong`     | Atari, 1972         | 1P (vs AI) and 2P modes               |
| `breakout` | Atari, 1976         | Speed ramps with each brick           |
| `snake`    | classic, 1976       | 1P or competitive 2P                  |
| `etch`     | sketch-pad genre    | Two-axis drawing toy                  |

## Embedding in another egui app

Drop the runner feature and pull `arcade-cart` as a normal egui dependency:

```toml
arcade-cart = { git = "https://github.com/privitera/shareware", default-features = false }
skifree     = { git = "https://github.com/privitera/shareware" }
# ...
```

Build a `Console` and call its `Game` impl from your own egui frame:

```rust
use arcade_cart::{Console, ConsoleConfig, Game};

let games: Vec<Box<dyn Game>> = vec![
    Box::new(skifree::SkiFree::new()),
    Box::new(pong::Pong::new()),
    // ...
];

let mut console = Console::new(games, ConsoleConfig {
    title: "MY ARCADE".into(),
    ..Default::default()
});

// Each frame, in your egui app:
console.update(dt, &input);  // input is your arcade_cart::Input
console.render(ui);
```

`ConsoleConfig` controls the home-screen text. Game titles, descriptions,
and years come from `Game::name() / description() / year()` on each cart.

## Native design target

All carts and the home screen are authored against a **1080×1920 portrait**
design rect (exposed as `arcade_cart::DESIGN_WIDTH` / `DESIGN_HEIGHT`).
Rendering at exactly that size is identity-scale; the standalone runner uses
540×960 (half-scale portrait) by default and the render code scales linearly.
A unit test in `arcade-cart` pins the identity-scale invariant so embedding
hosts can rely on pixel-for-pixel parity at the native target.

## Workspace layout

```
arcade/
├── README.md
├── Cargo.toml             # workspace
├── rust-toolchain.toml    # pinned to 1.94 (egui 0.34 MSRV)
│
├── arcade-cart/           # the spec: trait + Console + runner
│   └── src/
│       ├── lib.rs         # public exports + DESIGN_WIDTH / DESIGN_HEIGHT
│       ├── cart.rs        # Game trait, Input, Rng
│       ├── config.rs      # ConsoleConfig
│       ├── menu.rs        # home screen (CRT, glitch, starfield, scanlines)
│       ├── shutdown.rs    # CRT power-off animation
│       ├── console.rs     # state machine wrapping Vec<Box<dyn Game>>
│       └── runner.rs      # eframe shells, feature-gated behind `runner`
│
├── skifree/               # cart: ski simulator with yeti
│   ├── NOTICE             # Pirih + basicallydan/skifree.js attribution
│   ├── assets/*.png       # embedded via include_image!
│   └── src/{lib.rs, main.rs}
├── pong/
├── breakout/
├── snake/
├── etch/
│
└── cabinet/               # bin: bundles all five carts
    └── src/main.rs
```

## Roadmap

- [ ] **Full theming / customization API** — currently `ConsoleConfig`
      controls home-screen *text* only. A future revision will expose theme
      palette overrides, font selection, layout/sizing knobs, optional
      banner art, and per-game splash sprites.
- [ ] Keyboard binding overrides in the runner.
- [ ] Touch input mapping (mouse/touch drag → rotation).
- [ ] Save state per-cart (high scores).

## License

MIT — see individual `NOTICE` files in each cart for any third-party
attribution. Sprite assets in `skifree/` derive from
[`basicallydan/skifree.js`](https://github.com/basicallydan/skifree.js)
(MIT) and are distributed in homage to Chris Pirih's original 1991 game.
