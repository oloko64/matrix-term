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

    /// Printable ASCII only: letters, digits and symbols (no space).
    pub fn ascii() -> Self {
        Self {
            glyphs: ('!'..='~').collect(),
        }
    }

    pub fn random<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        *self.glyphs.choose(rng).expect("charset must not be empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_charset_is_printable_ascii_only() {
        let set = Charset::ascii();
        assert!(set.glyphs.iter().all(|c| c.is_ascii_graphic()));
        assert_eq!(set.glyphs.len(), 94);
    }

    #[test]
    fn matrix_charset_has_katakana() {
        assert!(Charset::matrix().glyphs.contains(&'\u{FF71}'));
    }
}
