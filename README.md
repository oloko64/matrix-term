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
- 🕶️ **Hacking mode** (`--glitch`): noise bursts, mutating glyphs,
  *SYSTEM BREACH* alerts and cyberpunk neon banners, RGB-split text
  and hacking feeds, randomly rotating
- 🛟 **Safe exit**: your terminal is restored on quit, *even after a panic*

## 🚀 Getting started

You need a recent [Rust toolchain](https://rustup.rs) (edition 2024).

```sh
git clone https://github.com/oloko64/matrix-term.git
cd matrix-term
cargo run --release
```

Or install it so it's always one command away. The installed command is **`mterm`**:

```sh
cargo install --path .
mterm
```

## 🎮 Controls

| Key | Action |
|---|---|
| `q` / `Esc` / `Ctrl+C` | Exit the Matrix |

> 💡 Use a terminal with **truecolor** support and a font that has
> half-width katakana for the best look (or use `--charset ascii`).

## ⚙️ Options

```sh
mterm --help
```

| Flag | Default | What it does |
|---|---|---|
| `--fps <N>` | `30` | Frames per second (1–240) |
| `-s, --spacing <N>` | `2` | Gap between rain columns (`1` = densest) |
| `--min-trail <N>` | `6` | Shortest trail, in rows |
| `--max-trail <N>` | `30` | Longest trail, in rows |
| `--min-speed <X>` | `0.3` | Slowest fall speed, in rows per frame |
| `--max-speed <X>` | `1` | Fastest fall speed, in rows per frame |
| `--max-delay <N>` | `90` | Max frames a column waits before dropping again |
| `-c, --color <HEX>` | `#00ff46` | Trail color (fades toward the tail) |
| `--head-color <HEX>` | `#dcffdc` | Color of the leading glyph |
| `--charset <SET>` | `matrix` | Glyphs: `matrix` (katakana + digits + symbols) or `ascii` (printable ASCII only) |
| `-g, --glitch` | off | 🕶️ Hacking mode: random glitch events while the rain falls |
| `--glitch-intensity <X>` | `0.5` | How often / how strong glitches are, `0.0` (rare) to `1.0` (chaos) |
| `--glitch-effects <LIST>` | all but `color` | Effects to rotate between (see below) |

### 🕶️ Glitch effects

| Effect | Default | Look |
|---|---|---|
| `noise` | ✅ | Bands of crackling glyphs tear across the screen |
| `mutate` | ✅ | Glyphs change while they fall |
| `message` | ✅ | Blinking red *SYSTEM BREACH*-style alert |
| `banner` | ✅ | Cyberpunk neon box banner with jittering, RGB-split slices |
| `chroma` | ✅ | Cyberpunk message split into magenta / white / cyan copies |
| `feed` | ✅ | Cyberpunk panel that types out a hacking log with a progress bar |
| `color` | ❌ | Trail glyphs flash red/white (opt-in) |

### Examples

```sh
# Dense, fast downpour
mterm -s 1 --min-speed 0.8 --max-speed 2

# Red pill 💊
mterm -c '#ff2050' --head-color '#ffffff'

# Plain ASCII, for fonts without katakana
mterm --charset ascii

# Someone is hacking the system 🕶️
mterm --glitch --glitch-intensity 0.8

# Only the cyberpunk effects
mterm -g --glitch-effects banner,chroma,feed

# Opt in to the color flashes too
mterm -g --glitch-effects noise,color,message

# Calm, long trails
mterm --max-speed 0.5 --min-trail 20 --max-trail 50
```

## 🧩 Architecture

The project is split so new features touch as few places as possible:

```
src/
├── main.rs      → wiring only
├── app.rs       → main loop: update → render → input
├── cli.rs       → command-line flags (clap) → Config
├── config.rs    → every tunable setting + defaults
├── glitch.rs    → hacking mode: glitch events (pure logic, unit tested)
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

- [x] Command-line flags (speed, density, colors, fps)
- [ ] Color themes
- [x] Glyphs that mutate inside the trail
- [x] Hacking / glitch mode
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
