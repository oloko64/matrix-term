//! Glyph pools for the rain.

use rand::{Rng, seq::IndexedRandom};

#[derive(Debug, Clone)]
pub struct Charset {
    glyphs: Vec<char>,
}

impl Charset {
    /// Half-width katakana + digits + a few symbols, like the movie.
    pub fn matrix() -> Self {
        let katakana = ('\u{FF66}'..='\u{FF9D}').collect::<Vec<_>>();
        let digits = '0'..='9';
        let symbols = [':', '.', '"', '=', '*', '+', '-', '<', '>', '|', 'Z'];
        let glyphs = katakana.into_iter().chain(digits).chain(symbols).collect();
        Self { glyphs }
    }

    pub fn random<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        *self.glyphs.choose(rng).expect("charset must not be empty")
    }
}
