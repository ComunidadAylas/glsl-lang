//! Last stage lexer declaration

use arrayvec::ArrayVec;

use crate::{
    lexer::{PreLexer, PreTextToken, PreToken as InputToken},
    util::LineMap,
};

mod token;
use lang_util::TextRange;
pub use token::Token;

pub type TextToken = crate::util::TextToken<token::Token>;

/// Final stage lexer.
///
/// This lexer wraps earlier stages and glues punctuation together to form (longest)
/// multi-character operator tokens. This is the entry point for lexing a pre-processor token
/// stream for the GLSL language.
#[derive(Debug, Clone)]
pub struct Lexer<'i> {
    input: PreLexer<'i>,
    /// Unglued token buffer. Since we're pasting at most 3 tokens together, and we always return
    /// one, we only need space for storing 2 pending tokens.
    buffer: ArrayVec<PreTextToken, 2>,
}

impl<'i> Lexer<'i> {
    pub fn new(input: &'i str) -> Self {
        Self {
            input: PreLexer::new(input),
            buffer: ArrayVec::new(),
        }
    }

    /// Get a reference to the input slice
    pub fn input(&self) -> &'i str {
        self.input.input()
    }

    /// Get a reference to the line map
    pub fn line_map(&self) -> &LineMap {
        self.input.line_map()
    }

    /// Consume this lexer and return the line map
    pub fn into_line_map(self) -> LineMap {
        self.input.into_line_map()
    }

    /// Notify the lexer we are parsing an #moj_import directive, and it should expect the next `<`
    /// token to start an angle-quoted string.
    ///
    /// # Parameters
    ///
    /// * `expect_angle_string`: true if the lexer should expect a string, false otherwise
    pub fn set_expect_angle_string(&mut self, expect_angle_string: bool) {
        self.input.set_expect_angle_string(expect_angle_string)
    }

    fn next(&mut self) -> Option<PreTextToken> {
        self.buffer.pop().or_else(|| self.input.next())
    }

    fn maybe_concat(
        &mut self,
        token: PreTextToken,
        next: impl FnOnce(InputToken) -> Option<Token>,
    ) -> Option<TextToken> {
        if let Some(next_token) = self.next() {
            let result = next(*next_token);

            if let Some(result) = result {
                return Some(TextToken::new(
                    result,
                    TextRange::new(token.range.start(), next_token.range.end()),
                ));
            } else {
                // Put the token back into the buffer, it couldn't be combined
                self.buffer.push(next_token);
            }
        }

        Some(token.transmute())
    }

    fn maybe_concat2(
        &mut self,
        token: PreTextToken,
        next: impl FnOnce(InputToken) -> Option<Token>,
        after: impl FnOnce((InputToken, InputToken)) -> Option<Token>,
    ) -> Option<TextToken> {
        if let Some(next_token) = self.next() {
            // We have a 2nd token

            if let Some(after_token) = self.next() {
                // We have a 3rd token, try to combine the 3
                let result = after((*next_token, *after_token));

                if let Some(result) = result {
                    // We combined three tokens
                    return Some(TextToken::new(
                        result,
                        TextRange::new(token.range.start(), after_token.range.end()),
                    ));
                } else {
                    // We failed to combine three tokens, try to combine two and buffer the rest
                    self.buffer.push(after_token);
                    self.buffer.push(next_token);

                    return self.maybe_concat(token, next);
                }
            } else {
                // End of input, only 2 tokens
                // Push the token back into the buffer so maybe_concat can use it
                self.buffer.push(next_token);
                return self.maybe_concat(token, next);
            }
        }

        Some(token.transmute())
    }
}

impl<'i> Iterator for Lexer<'i> {
    type Item = TextToken;

    fn next(&mut self) -> Option<Self::Item> {
        use Token::*;

        let token = self.next();
        match token {
            Some(token) => match *token {
                InputToken::PLUS => self.maybe_concat(token, |input| match input {
                    InputToken::PLUS => Some(INC_OP),
                    InputToken::EQUAL => Some(ADD_ASSIGN),
                    _ => None,
                }),
                InputToken::DASH => self.maybe_concat(token, |input| match input {
                    InputToken::DASH => Some(DEC_OP),
                    InputToken::EQUAL => Some(SUB_ASSIGN),
                    _ => None,
                }),
                InputToken::SLASH => self.maybe_concat(token, |input| match input {
                    InputToken::EQUAL => Some(DIV_ASSIGN),
                    _ => None,
                }),
                InputToken::ASTERISK => self.maybe_concat(token, |input| match input {
                    InputToken::EQUAL => Some(MUL_ASSIGN),
                    _ => None,
                }),
                InputToken::PERCENT => self.maybe_concat(token, |input| match input {
                    InputToken::EQUAL => Some(MOD_ASSIGN),
                    _ => None,
                }),
                InputToken::LANGLE => self.maybe_concat2(
                    token,
                    |input| match input {
                        InputToken::LANGLE => Some(LEFT_OP),
                        InputToken::EQUAL => Some(LE_OP),
                        _ => None,
                    },
                    |input| match input {
                        (InputToken::LANGLE, InputToken::EQUAL) => Some(LEFT_ASSIGN),
                        _ => None,
                    },
                ),
                InputToken::RANGLE => self.maybe_concat2(
                    token,
                    |input| match input {
                        InputToken::RANGLE => Some(RIGHT_OP),
                        InputToken::EQUAL => Some(GE_OP),
                        _ => None,
                    },
                    |input| match input {
                        (InputToken::RANGLE, InputToken::EQUAL) => Some(RIGHT_ASSIGN),
                        _ => None,
                    },
                ),
                InputToken::CARET => self.maybe_concat(token, |input| match input {
                    InputToken::CARET => Some(XOR_OP),
                    InputToken::EQUAL => Some(XOR_ASSIGN),
                    _ => None,
                }),
                InputToken::BAR => self.maybe_concat(token, |input| match input {
                    InputToken::BAR => Some(OR_OP),
                    InputToken::EQUAL => Some(OR_ASSIGN),
                    _ => None,
                }),
                InputToken::AMPERSAND => self.maybe_concat(token, |input| match input {
                    InputToken::AMPERSAND => Some(AND_OP),
                    InputToken::EQUAL => Some(AND_ASSIGN),
                    _ => None,
                }),
                InputToken::EQUAL => self.maybe_concat(token, |input| match input {
                    InputToken::EQUAL => Some(EQ_OP),
                    _ => None,
                }),
                InputToken::BANG => self.maybe_concat(token, |input| match input {
                    InputToken::EQUAL => Some(NE_OP),
                    _ => None,
                }),
                InputToken::HASH => self.maybe_concat(token, |input| match input {
                    InputToken::HASH => Some(PP_CONCAT),
                    _ => None,
                }),
                InputToken::PERIOD => self.maybe_concat(token, |input| match input {
                    InputToken::DIGITS => Some(DIGITS),
                    _ => None,
                }),
                _ => Some(token.transmute()),
            },
            None => None,
        }
    }
}
