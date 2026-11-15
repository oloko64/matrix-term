//! "Hacking" glitch mode: a small state machine that, once per frame,
//! describes which glitch effects to apply. Pure logic, no terminal I/O:
//! overlays are returned as styled text spans for the renderer to paint.

use rand::{RngExt, rngs::ThreadRng, seq::IndexedRandom};

use crate::{charset::Charset, config::Rgb};

/// The kinds of glitch events. Each event picks one at random.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Horizontal bands of crackling noise glyphs.
    Noise,
    /// Trail glyphs flash in alert colors.
    Color,
    /// Glyphs inside the trails change while falling.
    Mutate,
    /// A blinking red "breach" message in the middle of the screen.
    Message,
    /// Cyberpunk: a neon boxed banner with jittering slices.
    Banner,
    /// Cyberpunk: message with an RGB split (magenta / white / cyan).
    Chroma,
    /// Cyberpunk: a panel that types out a hacking log.
    Feed,
}

impl Kind {
    #[cfg(test)]
    pub const ALL: [Kind; 7] = [
        Kind::Noise,
        Kind::Color,
        Kind::Mutate,
        Kind::Message,
        Kind::Banner,
        Kind::Chroma,
        Kind::Feed,
    ];

    /// Event duration range, in seconds (before intensity scaling).
    /// `Feed` computes its own duration from its content.
    fn duration_secs(self) -> (f32, f32) {
        match self {
            Kind::Noise => (0.15, 0.5),
            Kind::Color => (0.3, 0.8),
            Kind::Mutate => (0.5, 1.5),
            Kind::Message => (1.0, 2.0),
            Kind::Banner => (1.2, 2.2),
            Kind::Chroma => (1.0, 2.0),
            Kind::Feed => (0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlitchConfig {
    /// 0.0 = rare and subtle, 1.0 = chaos.
    pub intensity: f32,
    /// Effects events are picked from. Must not be empty to do anything.
    pub effects: Vec<Kind>,
}

/// How a span of overlay text is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Rgb,
    pub bg: Option<Rgb>,
    pub bold: bool,
}

impl Style {
    const fn fg(fg: Rgb) -> Self {
        Self {
            fg,
            bg: None,
            bold: false,
        }
    }

    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    const fn on(mut self, bg: Rgb) -> Self {
        self.bg = Some(bg);
        self
    }
}

/// A run of styled text drawn on top of the rain. Always fits the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub row: u16,
    pub col: u16,
    pub text: Vec<char>,
    pub style: Style,
}

/// What to do this frame. `Frame::default()` means "no glitch".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// Overlay text, drawn after the rain.
    pub spans: Vec<Span>,
    /// Chance (0..=1) that a drawn trail cell uses an alert color.
    pub corrupt_chance: f32,
    /// Chance (0..=1) that each trail glyph changes this frame.
    pub mutate_chance: f32,
    /// Rows to wipe before drawing (overlay leftovers).
    pub clear_rows: Vec<u16>,
    /// Repaint every falling trail, not just the ones that moved.
    pub force_redraw: bool,
}

mod palette {
    use crate::config::Rgb;

    pub const BLACK: Rgb = Rgb(0, 0, 0);
    pub const WHITE: Rgb = Rgb(255, 255, 255);
    pub const ALERT_RED: Rgb = Rgb(255, 40, 60);
    pub const NOISE_BRIGHT: Rgb = Rgb(230, 255, 230);
    pub const NOISE_DIM: Rgb = Rgb(90, 110, 90);
    pub const NEON_YELLOW: Rgb = Rgb(252, 238, 10);
    pub const NEON_CYAN: Rgb = Rgb(0, 240, 255);
    pub const NEON_MAGENTA: Rgb = Rgb(255, 0, 170);
    pub const DIM_CYAN: Rgb = Rgb(0, 120, 140);
}
use palette::*;

const PHRASES: &[&str] = &[
    "SYSTEM BREACH",
    "ACCESS GRANTED",
    "INTRUSION DETECTED",
    "CONNECTION TRACED",
    "FIREWALL DOWN",
    "ROOT ACCESS OBTAINED",
    "DECRYPTING...",
];

const CYBER_PHRASES: &[&str] = &[
    "NETWORK INTRUSION DETECTED",
    "ACCESS VIOLATION",
    "NEURAL LINK COMPROMISED",
    "SECURITY OVERRIDE",
    "SYSTEM CORRUPTED",
    "UNAUTHORIZED ACCESS",
];

const FEED_STEPS: &[&str] = &[
    "> bypassing firewall ....... OK",
    "> spoofing credentials ..... OK",
    "> cracking ICE layer ....... OK",
    "> disabling trace .......... OK",
    "> escalating privileges .... OK",
    "> decrypting keystore ...... OK",
];

const FEED_BAR_LABEL: &str = "> uploading payload ";
const FEED_BAR_WIDTH: usize = 12;

/// One line of the hacking feed.
enum FeedLine {
    Text(Vec<char>),
    /// A label followed by an animated progress bar.
    Bar(Vec<char>),
}

impl FeedLine {
    fn label_len(&self) -> usize {
        match self {
            FeedLine::Text(t) | FeedLine::Bar(t) => t.len(),
        }
    }
}

/// Per-event content, picked when the event starts.
enum Content {
    None,
    Phrase(&'static str),
    Banner {
        phrase: &'static str,
        sub: String,
        color: Rgb,
    },
    Feed {
        lines: Vec<FeedLine>,
        /// Typed characters per frame.
        speed: f32,
        /// Typing "cost" of the progress bar, in characters.
        bar_cost: f32,
    },
}

struct Active {
    kind: Kind,
    ticks_left: u32,
    tick: u32,
    /// First screen row of the overlay (may be negative on tiny screens).
    top: i32,
    /// Screen rows this event draws overlays on (wiped when it ends).
    rows: Vec<u16>,
    content: Content,
}

pub struct Glitch {
    cfg: GlitchConfig,
    fps: u32,
    cooldown: u32,
    active: Option<Active>,
    rng: ThreadRng,
}

impl Glitch {
    pub fn new(mut cfg: GlitchConfig, fps: u32) -> Self {
        cfg.intensity = cfg.intensity.clamp(0.0, 1.0);
        let mut glitch = Self {
            cfg,
            fps: fps.max(1),
            cooldown: 0,
            active: None,
            rng: rand::rng(),
        };
        glitch.reset();
        glitch
    }

    /// Cancels any running event and starts a fresh cooldown.
    pub fn reset(&mut self) {
        self.active = None;
        self.cooldown = self.next_cooldown();
    }

    #[cfg(test)]
    pub fn active_kind(&self) -> Option<Kind> {
        self.active.as_ref().map(|a| a.kind)
    }

    /// Mean gap between events: ~8s at intensity 0, ~1s at intensity 1.
    fn mean_gap_secs(&self) -> f32 {
        8.0 - 7.0 * self.cfg.intensity
    }

    /// Longest possible cooldown, in frames.
    #[cfg(test)]
    pub fn max_cooldown(&self) -> u32 {
        self.ticks(self.mean_gap_secs() * 1.5)
    }

    fn ticks(&self, secs: f32) -> u32 {
        (secs * self.fps as f32).round().max(1.0) as u32
    }

    fn next_cooldown(&mut self) -> u32 {
        let mean = self.mean_gap_secs();
        let secs = self.rng.random_range(mean * 0.5..=mean * 1.5);
        self.ticks(secs)
    }

    /// Advances one frame and returns the effects to apply.
    pub fn update(&mut self, width: u16, height: u16, charset: &Charset) -> Frame {
        let mut fx = Frame::default();
        if width == 0 || height == 0 {
            return fx;
        }

        if self.active.is_none() {
            if self.cooldown > 0 {
                self.cooldown -= 1;
                return fx;
            }
            match self.start(height) {
                Some(active) => self.active = Some(active),
                None => {
                    self.cooldown = self.next_cooldown();
                    return fx;
                }
            }
        }

        let mut active = self.active.take().expect("checked above");
        if active.ticks_left == 0 {
            // Event over: repair the screen and go back to idle.
            fx.clear_rows = active.rows;
            fx.force_redraw = true;
            self.cooldown = self.next_cooldown();
            return fx;
        }
        active.ticks_left -= 1;
        active.tick += 1;
        let mut canvas = Canvas {
            fx: &mut fx,
            width,
            height,
        };
        self.fill(&active, &mut canvas, charset);
        self.active = Some(active);
        fx
    }

    fn start(&mut self, height: u16) -> Option<Active> {
        let kind = *self.cfg.effects.choose(&mut self.rng)?;
        let intensity = self.cfg.intensity;
        let h = height as i32;
        let scale = 0.7 + 0.6 * intensity;

        let (content, overlay_rows, mut ticks_left) = match kind {
            Kind::Noise => {
                let bands = 1 + (intensity * 3.0).round() as usize;
                let mut rows: Vec<i32> = (0..bands).map(|_| self.rng.random_range(0..h)).collect();
                rows.sort_unstable();
                rows.dedup();
                (Content::None, rows, 0)
            }
            Kind::Color | Kind::Mutate => (Content::None, Vec::new(), 0),
            Kind::Message => (Content::Phrase(self.pick(PHRASES)), vec![h / 2], 0),
            Kind::Banner => {
                let content = Content::Banner {
                    phrase: self.pick(CYBER_PHRASES),
                    sub: format!(
                        "sys://node_{:02} :: 0x{:04X}",
                        self.rng.random_range(0..100u32),
                        self.rng.random_range(0..=u16::MAX)
                    ),
                    color: *[NEON_YELLOW, NEON_CYAN, NEON_MAGENTA]
                        .choose(&mut self.rng)
                        .expect("non-empty"),
                };
                // Noise strip + 4 box rows.
                (content, rows_from(h / 2 - 3, 5), 0)
            }
            Kind::Chroma => {
                // Magenta, white, cyan copies + scanline.
                (
                    Content::Phrase(self.pick(CYBER_PHRASES)),
                    rows_from(h / 2 - 1, 4),
                    0,
                )
            }
            Kind::Feed => {
                let content = self.feed_content();
                let Content::Feed {
                    lines,
                    speed,
                    bar_cost,
                } = &content
                else {
                    unreachable!()
                };
                let cost: usize = lines.iter().map(FeedLine::label_len).sum();
                let typing = ((cost as f32 + bar_cost) / speed).ceil() as u32;
                let rows = rows_from(h / 2 - (lines.len() as i32 + 2) / 2, lines.len() + 2);
                let hold = self.ticks(1.2);
                (content, rows, typing + hold)
            }
        };

        if ticks_left == 0 {
            let (lo, hi) = kind.duration_secs();
            let secs = self.rng.random_range(lo..=hi) * scale;
            ticks_left = self.ticks(secs);
        }
        let top = overlay_rows.first().copied().unwrap_or(0);
        let rows = overlay_rows
            .into_iter()
            .filter(|r| (0..h).contains(r))
            .map(|r| r as u16)
            .collect();

        Some(Active {
            kind,
            ticks_left,
            tick: 0,
            top,
            rows,
            content,
        })
    }

    fn pick(&mut self, list: &'static [&'static str]) -> &'static str {
        list.choose(&mut self.rng)
            .copied()
            .unwrap_or("SYSTEM BREACH")
    }

    fn feed_content(&mut self) -> Content {
        let ip = format!(
            "> connecting {}.{}.{}.{} ..... OK",
            self.rng.random_range(10..=223u8),
            self.rng.random_range(0..=255u8),
            self.rng.random_range(0..=255u8),
            self.rng.random_range(1..=254u8)
        );
        let steps: Vec<&str> = FEED_STEPS.sample(&mut self.rng, 2).copied().collect();
        let chars = |s: &str| s.chars().collect::<Vec<_>>();
        let lines = vec![
            FeedLine::Text(chars(&ip)),
            FeedLine::Text(chars(steps[0])),
            FeedLine::Text(chars(steps[1])),
            FeedLine::Bar(chars(FEED_BAR_LABEL)),
            FeedLine::Text(chars("> ACCESS GRANTED")),
        ];
        let cps = 40.0 + 50.0 * self.cfg.intensity;
        Content::Feed {
            lines,
            speed: cps / self.fps as f32,
            bar_cost: cps * 0.8,
        }
    }

    /// Replaces non-space characters with random glyphs, with `chance`.
    fn corrupt_text(&mut self, text: &str, chance: f64, charset: &Charset) -> Vec<char> {
        text.chars()
            .map(|c| {
                if c != ' ' && self.rng.random_bool(chance) {
                    charset.random(&mut self.rng)
                } else {
                    c
                }
            })
            .collect()
    }

    fn fill(&mut self, a: &Active, canvas: &mut Canvas, charset: &Charset) {
        let intensity = self.cfg.intensity;
        let w = canvas.width as i32;
        match (&a.kind, &a.content) {
            (Kind::Noise, _) => {
                for &row in &a.rows {
                    let start = self.rng.random_range(0..w);
                    let len = self.rng.random_range(1..=w - start);
                    let glyphs = (0..len).map(|_| charset.random(&mut self.rng)).collect();
                    let color = if self.rng.random_bool(0.5) {
                        NOISE_BRIGHT
                    } else {
                        NOISE_DIM
                    };
                    canvas.span(row as i32, start, glyphs, Style::fg(color));
                }
            }
            (Kind::Color, _) => {
                canvas.fx.corrupt_chance = 0.15 + 0.35 * intensity;
                canvas.fx.force_redraw = true;
            }
            (Kind::Mutate, _) => {
                canvas.fx.mutate_chance = 0.1 + 0.3 * intensity;
                canvas.fx.force_redraw = true;
            }
            (Kind::Message, Content::Phrase(phrase)) => {
                canvas.wipe(&a.rows);
                // Blink: visible 4 frames out of 6.
                if a.tick % 6 >= 4 {
                    return;
                }
                let noise = 0.05 + 0.1 * f64::from(intensity);
                let text = self.corrupt_text(&format!(" {phrase} "), noise, charset);
                let col = (w - text.len() as i32) / 2;
                canvas.span(a.top, col, text, Style::fg(ALERT_RED).bold());
            }
            (Kind::Banner, Content::Banner { phrase, sub, color }) => {
                canvas.wipe(&a.rows);
                if self.rng.random_bool(0.08) {
                    return; // flicker off
                }
                self.fill_banner(a.top, phrase, sub, *color, canvas, charset);
            }
            (Kind::Chroma, Content::Phrase(phrase)) => {
                canvas.wipe(&a.rows);
                if self.rng.random_bool(0.08) {
                    return; // flicker off
                }
                self.fill_chroma(a.top, phrase, canvas, charset);
            }
            (
                Kind::Feed,
                Content::Feed {
                    lines,
                    speed,
                    bar_cost,
                },
            ) => {
                let typed = a.tick as f32 * speed;
                let blink = (a.tick / 8).is_multiple_of(2);
                fill_feed(a.top, lines, typed, *bar_cost, blink, canvas);
            }
            _ => {}
        }
    }

    fn fill_banner(
        &mut self,
        top: i32,
        phrase: &str,
        sub: &str,
        color: Rgb,
        canvas: &mut Canvas,
        charset: &Charset,
    ) {
        let intensity = f64::from(self.cfg.intensity);
        let title = format!(">> {phrase} <<");
        let inner = title.chars().count().max(sub.chars().count()) + 4;
        let center = |s: &str| {
            let n = s.chars().count();
            let left = (inner - n) / 2;
            format!("{}{s}{}", " ".repeat(left), " ".repeat(inner - n - left))
        };
        let lines = [
            format!("░▒▓ {} ▓▒░", "/".repeat(inner.saturating_sub(6))),
            format!("╔{}╗", "═".repeat(inner)),
            format!("║{}║", center(&title)),
            format!("║{}║", center(sub)),
            format!("╚{}╝", "═".repeat(inner)),
        ];
        let box_w = inner as i32 + 2;
        let col = (canvas.width as i32 - box_w) / 2;
        let jitter = 0.15 + 0.25 * intensity;
        let noise = 0.02 + 0.06 * intensity;

        for (i, line) in lines.iter().enumerate() {
            let row = top + i as i32;
            let text = self.corrupt_text(line, noise, charset);
            let strip = i == 0;
            if self.rng.random_bool(jitter) {
                // Slice tears sideways with an RGB-split color.
                let shift = *[-2, -1, 1, 2].choose(&mut self.rng).expect("non-empty");
                let fg = if self.rng.random_bool(0.5) {
                    NEON_CYAN
                } else {
                    ALERT_RED
                };
                canvas.span(row, col + shift, text, Style::fg(fg).on(BLACK).bold());
            } else if strip {
                canvas.span(row, col, text, Style::fg(color));
            } else {
                canvas.span(row, col, text, Style::fg(BLACK).on(color).bold());
            }
        }
    }

    fn fill_chroma(&mut self, top: i32, phrase: &str, canvas: &mut Canvas, charset: &Charset) {
        let intensity = f64::from(self.cfg.intensity);
        let n = phrase.chars().count() as i32;
        let col = (canvas.width as i32 - n) / 2;
        let noise = 0.03 + 0.08 * intensity;
        let spread = 1 + (intensity * 2.0).round() as i32;

        let left = self.rng.random_range(1..=spread);
        let right = self.rng.random_range(1..=spread);
        let wobble = if self.rng.random_bool(0.15) {
            *[-1, 1].choose(&mut self.rng).expect("non-empty")
        } else {
            0
        };

        let magenta = self.corrupt_text(phrase, noise, charset);
        let white = self.corrupt_text(phrase, noise / 3.0, charset);
        let cyan = self.corrupt_text(phrase, noise, charset);
        canvas.span(top, col - left, magenta, Style::fg(NEON_MAGENTA));
        canvas.span(top + 1, col + wobble, white, Style::fg(WHITE).bold());
        canvas.span(top + 2, col + right, cyan, Style::fg(NEON_CYAN));

        let scan_len = n + 8;
        let scan: Vec<char> = (0..scan_len)
            .map(|_| {
                if self.rng.random_bool(0.15) {
                    '/'
                } else {
                    '─'
                }
            })
            .collect();
        canvas.span(top + 3, col - 4, scan, Style::fg(DIM_CYAN));
    }
}

/// Lays out the hacking feed panel for a given amount of typed characters.
fn fill_feed(
    top: i32,
    lines: &[FeedLine],
    typed: f32,
    bar_cost: f32,
    blink: bool,
    canvas: &mut Canvas,
) {
    const TITLE: &str = "─[ REMOTE SHELL ]";
    let bar_len = FEED_BAR_LABEL.chars().count() + FEED_BAR_WIDTH + 2 + 5;
    let inner = lines
        .iter()
        .map(|l| match l {
            FeedLine::Text(t) => t.len(),
            FeedLine::Bar(_) => bar_len,
        })
        .max()
        .unwrap_or(0)
        + 1; // room for the cursor
    let box_w = inner as i32 + 4;
    let col = (canvas.width as i32 - box_w) / 2;
    let border = Style::fg(NEON_YELLOW);

    let title_len = TITLE.chars().count();
    let top_line = format!(
        "┌{TITLE}{}┐",
        "─".repeat((inner + 2).saturating_sub(title_len))
    );
    canvas.span(top, col, top_line.chars().collect(), border.bold());

    let mut budget = typed;
    let mut cursor_drawn = false;
    for (i, line) in lines.iter().enumerate() {
        let row = top + 1 + i as i32;
        let mut text: Vec<char> = Vec::new();
        let done = match line {
            FeedLine::Text(t) => {
                let n = (budget.max(0.0) as usize).min(t.len());
                text.extend(&t[..n]);
                budget -= t.len() as f32;
                budget >= 0.0
            }
            FeedLine::Bar(label) => {
                let n = (budget.max(0.0) as usize).min(label.len());
                text.extend(&label[..n]);
                budget -= label.len() as f32;
                if budget > 0.0 {
                    let frac = (budget / bar_cost).clamp(0.0, 1.0);
                    let filled = (frac * FEED_BAR_WIDTH as f32).round() as usize;
                    text.push('[');
                    text.extend(std::iter::repeat_n('█', filled));
                    text.extend(std::iter::repeat_n('░', FEED_BAR_WIDTH - filled));
                    text.extend(format!("] {:>3}%", (frac * 100.0).round() as u32).chars());
                }
                budget -= bar_cost;
                budget >= 0.0
            }
        };
        let last = i + 1 == lines.len();
        if !cursor_drawn && (!done || last) {
            cursor_drawn = true;
            if blink {
                text.push('█');
            }
        }
        text.resize(inner, ' ');

        let style = if last {
            Style::fg(NEON_YELLOW).bold()
        } else {
            Style::fg(NEON_CYAN)
        };
        canvas.span(row, col, vec!['│', ' '], border);
        canvas.span(row, col + 2, text, style);
        canvas.span(row, col + 2 + inner as i32, vec![' ', '│'], border);
    }

    let bottom = format!("└{}┘", "─".repeat(inner + 2));
    canvas.span(
        top + 1 + lines.len() as i32,
        col,
        bottom.chars().collect(),
        border,
    );
}

/// `count` consecutive rows starting at `top`.
fn rows_from(top: i32, count: usize) -> Vec<i32> {
    (top..top + count as i32).collect()
}

/// Adds spans to a frame, clipping them to the screen.
struct Canvas<'a> {
    fx: &'a mut Frame,
    width: u16,
    height: u16,
}

impl Canvas<'_> {
    fn span(&mut self, row: i32, col: i32, text: Vec<char>, style: Style) {
        let (w, h) = (self.width as i32, self.height as i32);
        if !(0..h).contains(&row) || col >= w {
            return;
        }
        let skip = (-col).max(0) as usize;
        let col = col.max(0);
        let text: Vec<char> = text
            .into_iter()
            .skip(skip)
            .take((w - col) as usize)
            .collect();
        if !text.is_empty() {
            self.fx.spans.push(Span {
                row: row as u16,
                col: col as u16,
                text,
                style,
            });
        }
    }

    /// Clears the overlay rows this frame (for overlays that move or blink).
    fn wipe(&mut self, rows: &[u16]) {
        self.fx.clear_rows.extend_from_slice(rows);
        self.fx.force_redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glitch(intensity: f32, effects: &[Kind]) -> Glitch {
        let cfg = GlitchConfig {
            intensity,
            effects: effects.to_vec(),
        };
        Glitch::new(cfg, 30)
    }

    #[test]
    fn event_starts_within_max_cooldown() {
        let charset = Charset::matrix();
        let mut g = glitch(1.0, &Kind::ALL);
        for _ in 0..=g.max_cooldown() {
            g.update(80, 24, &charset);
            if g.active_kind().is_some() {
                return;
            }
        }
        panic!("no glitch event started");
    }

    #[test]
    fn only_enabled_kinds_run() {
        let charset = Charset::matrix();
        let mut g = glitch(1.0, &[Kind::Color]);
        let mut seen = false;
        for _ in 0..3000 {
            g.update(80, 24, &charset);
            if let Some(kind) = g.active_kind() {
                assert_eq!(kind, Kind::Color);
                seen = true;
            }
        }
        assert!(seen);
    }

    #[test]
    fn no_effects_means_no_events() {
        let charset = Charset::matrix();
        let mut g = glitch(1.0, &[]);
        for _ in 0..3000 {
            assert_eq!(g.update(80, 24, &charset), Frame::default());
        }
    }

    #[test]
    fn overlay_events_end_by_clearing_their_rows() {
        let charset = Charset::matrix();
        for kind in [
            Kind::Noise,
            Kind::Message,
            Kind::Banner,
            Kind::Chroma,
            Kind::Feed,
        ] {
            let mut g = glitch(1.0, &[kind]);
            let mut drawn_rows = Vec::new();
            let mut ended = false;
            for _ in 0..5000 {
                let was_active = g.active_kind().is_some();
                let fx = g.update(80, 24, &charset);
                drawn_rows.extend(fx.spans.iter().map(|s| s.row));
                if was_active && g.active_kind().is_none() {
                    assert!(fx.force_redraw, "{kind:?}");
                    assert!(
                        drawn_rows.iter().all(|r| fx.clear_rows.contains(r)),
                        "{kind:?} left rows behind"
                    );
                    ended = true;
                    break;
                }
            }
            assert!(ended, "{kind:?} never ended");
            assert!(!drawn_rows.is_empty(), "{kind:?} drew nothing");
        }
    }

    #[test]
    fn feed_finishes_typing_before_it_ends() {
        let charset = Charset::matrix();
        let mut g = glitch(0.5, &[Kind::Feed]);
        let mut last_frame = Frame::default();
        for _ in 0..5000 {
            let was_active = g.active_kind().is_some();
            let fx = g.update(80, 24, &charset);
            if was_active && g.active_kind().is_none() {
                let text: String = last_frame
                    .spans
                    .iter()
                    .flat_map(|s| s.text.iter())
                    .collect();
                assert!(text.contains("ACCESS GRANTED"), "{text}");
                assert!(text.contains("100%"), "{text}");
                return;
            }
            last_frame = fx;
        }
        panic!("feed never ended");
    }

    #[test]
    fn overlays_stay_inside_the_screen() {
        let charset = Charset::matrix();
        for (w, h) in [(80, 24), (20, 6), (5, 3), (1, 1)] {
            let mut g = glitch(1.0, &Kind::ALL);
            for _ in 0..3000 {
                let fx = g.update(w, h, &charset);
                for span in &fx.spans {
                    assert!(span.row < h);
                    assert!(!span.text.is_empty());
                    assert!(span.col as usize + span.text.len() <= w as usize);
                }
                assert!(fx.clear_rows.iter().all(|&r| r < h));
            }
        }
    }
}
