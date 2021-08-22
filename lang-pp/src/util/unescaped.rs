use std::{
    borrow::Cow,
    fmt::Write,
    iter::Peekable,
    str::{CharIndices, MatchIndices},
};

use arrayvec::ArrayVec;
use smol_str::SmolStr;

#[derive(Debug, Clone, Copy)]
pub struct Unescaped<'s> {
    src: &'s str,
}

impl<'s> Unescaped<'s> {
    pub fn new(src: &'s str) -> Self {
        Self { src }
    }

    fn backslashes(&self) -> MatchIndices<'s, char> {
        self.src.match_indices('\\')
    }

    pub fn chars(&self) -> UnescapeIter<'s> {
        UnescapeIter {
            chars: self.src.char_indices(),
            backslashes: self.backslashes().peekable(),
        }
    }

    pub fn to_string(self) -> Cow<'s, str> {
        if self.backslashes().next().is_none() {
            Cow::Borrowed(self.src)
        } else {
            Cow::Owned(self.chars().collect::<String>())
        }
    }
}

impl<'s> From<Unescaped<'s>> for SmolStr {
    fn from(src: Unescaped<'s>) -> Self {
        src.chars().collect()
    }
}

impl<'s> PartialEq<&str> for Unescaped<'s> {
    fn eq(&self, other: &&str) -> bool {
        self.chars().eq(other.chars())
    }
}

impl<'s> std::fmt::Display for Unescaped<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ch in self.chars() {
            f.write_char(ch)?;
        }

        Ok(())
    }
}

pub struct UnescapeIter<'s> {
    chars: CharIndices<'s>,
    backslashes: Peekable<MatchIndices<'s, char>>,
}

impl<'s> Iterator for UnescapeIter<'s> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((cont, _)) = self.backslashes.peek() {
                // There is a continuation coming

                // Create a peekable chars iterator
                let mut chars_copy = self.chars.clone().peekable();

                if let Some((i, _)) = chars_copy.peek() {
                    if *i == *cont {
                        // Consume this backslash match
                        self.backslashes.next();

                        // We are at the start of a potential continuation
                        // Skip 1 char (the backslash character)
                        // Collect 2 chars (worst case for a Windows CRLF)
                        let chars: ArrayVec<_, 2> =
                            chars_copy.map(|(_, ch)| ch).skip(1).take(2).collect();

                        // Consume the backslash char
                        self.chars.next();

                        if chars.starts_with(&['\r', '\n']) || chars.starts_with(&['\n', '\r']) {
                            // CRLF, advance thrice, loop again
                            self.chars.next(); // \r
                            self.chars.next(); // \n
                        } else if chars.starts_with(&['\n']) || chars.starts_with(&['\r']) {
                            // LF, advance twice, loop again
                            self.chars.next(); // \n
                        } else {
                            // Stray backslash, just return as-is
                            return Some('\\');
                        }
                    } else {
                        // We haven't reached the continuation yet
                        return self.chars.next().map(|(_, ch)| ch);
                    }
                } else {
                    // Nothing left
                    return None;
                }
            } else {
                // No continuation, i.e. happy path
                return self.chars.next().map(|(_, ch)| ch);
            }
        }
    }
}