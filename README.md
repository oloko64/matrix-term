<div align="center">

```
ﾊ 7 ｿ ﾘ   ｦ  ﾐ 4 ﾂ   ﾃ ﾅ
ﾐ   ﾈ ｷ   ﾛ  ｱ   ﾓ   ﾋ ｽ
  m a t r i x - t e r m
ｳ   ｼ ﾅ   ﾜ  2   ｹ   ﾒ ｴ
ｸ 0   ｵ   ﾗ  ﾁ   ﾄ   ﾉ
```

**Wake up, Neo… your terminal has the Matrix.**

Digital rain for your terminal, written in Rust 🦀

![Rust](https://img.shields.io/badge/rust-2024-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-green)

</div>

---

## ✨ Features

- 🟩 **Full-screen digital rain** in half-width katakana, digits and symbols
- ⚪ **Glowing heads** with green trails that fade into the dark
- 🎲 **Every drop is unique**: random speed, length and spawn delay
- 🧊 **No flicker**: only cells that changed are redrawn, never a full clear
- 📐 **Resize-aware**: the rain rebuilds itself when the window changes
- 🛟 **Safe exit**: your terminal is restored on quit, *even after a panic*

## 🚀 Getting started

You need a recent [Rust toolchain](https://rustup.rs) (edition 2024).

```sh
git clone https://github.com/oloko64/matrix-term.git
cd matrix-term
cargo run --release
```

Or install it so it's always one command away:

```sh
cargo install --path .
matrix-term
```

## 🎮 Controls

| Key | Action |
|---|---|
| `q` / `Esc` / `Ctrl+C` | Exit the Matrix |

> 💡 Use a terminal with **truecolor** support and a font that has
> half-width katakana for the best look.

## ⚙️ Configuration

For now, settings live in [`src/config.rs`](src/config.rs):

| Setting | Default | What it does |
|---|---|---|
| `fps` | `30` | Frames per second |
| `column_spacing` | `2` | Gap between rain columns (`1` = densest) |
| `trail_len` | `6..=30` | Trail length range, in rows |
| `speed` | `0.3..=1.0` | Fall speed range, in rows per frame |
| `max_spawn_delay` | `90` | Max frames a column waits before a new drop |
| `head_color` | `Rgb(220, 255, 220)` | Color of the leading glyph |
| `trail_color` | `Rgb(0, 255, 70)` | Trail color (fades toward the tail) |
| `charset` | `Charset::matrix()` | Glyphs the rain is made of |

## 🧩 Architecture

The project is split so new features touch as few places as possible:

```
src/
├── main.rs      → wiring only
├── app.rs       → main loop: update → render → input
├── config.rs    → every tunable setting
├── charset.rs   → glyph pools
├── rain.rs      → simulation (pure logic, unit tested)
├── render.rs    → draws the rain with crossterm
├── input.rs     → keys & resize → actions
└── terminal.rs  → raw mode / alt screen guard + panic hook
```

The simulation (`rain.rs`) knows nothing about the terminal, and the
renderer (`render.rs`) knows nothing about how drops move. Want a new
color theme? Touch `config.rs`/`render.rs`. New rain behaviour? Touch
`rain.rs`.

## 🗺️ Roadmap

- [ ] Command-line flags (speed, density, colors, fps)
- [ ] Color themes
- [ ] Glyphs that mutate inside the trail
- [ ] Pause / runtime speed & density controls
- [ ] Hidden message mode

## 🧪 Development

```sh
cargo test
cargo clippy --all-targets
```

## 📜 License

[MIT](LICENSE) © 2026 OLoKo64

<div align="center">

*There is no spoon.* 🥄

</div>
