# shareware

A shelf of nostalgic computing curios by [1337_Pete](https://privitera.github.io/ppplugins/).

## Contents

| Item | Description |
|------|-------------|
| [`ssstars.scr`](./ssstars.scr) | Classic Windows Starfield screensaver |
| [`arcade/`](./arcade) | Cartridge-style retro arcade — five mini-games + console + CRT shutdown, in Rust |

## ssstars.scr

Drop it in `C:\Windows\System32\`, then go to your screensaver options — you'll see **Starfield** available. Enjoy.

## arcade

A small Rust workspace with a console-and-cartridge architecture. Five carts ship out of the box: SkiFree, Pong, Breakout, Snake, and a sketch pad. Each runs standalone, or all five together in a bundled cabinet with a home screen and a satisfying CRT shutdown when you exit.

```bash
cd arcade
cargo run -p cabinet     # the full console
cargo run -p skifree     # one cart standalone
```

See [`arcade/README.md`](./arcade/README.md) for the full architecture, embedding guide, and roadmap.

---

Just nostalgia. Run binaries from the internet at your own risk.
