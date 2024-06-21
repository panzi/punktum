// trying to emulate: https://github.com/theskumar/python-dotenv/blob/main/src/dotenv/parser.py
use std::{io::BufRead, path::Path};

use crate::{Env, Error, Options, Result, DEBUG_PREFIX};

pub fn config_python_dotenv(reader: &mut dyn BufRead, env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
    let mut string = String::new();
    options.encoding.read_to_string(reader, &mut string)?;
    let mut reader = Reader::new(string);

    while reader.has_next() {
        let Some(binding) = parse_binding(&mut reader)? else {
            continue;
        };
        let value = if let Some(value) = &binding.value {
            value
        } else {
            ""
        };
        options.set_var_cut_null(env, &binding.key, value);
    }
    
    Ok(())
}

struct Position {
    index: usize,
    lineno: usize,
}

impl Position {
    #[inline]
    pub fn new(index: usize, line: usize) -> Self {
        Self { index, lineno: line }
    }

    #[inline]
    pub fn start() -> Self {
        Self { index: 0, lineno: 0 }
    }

    #[inline]
    pub fn set(&mut self, other: &Position) {
        self.index = other.index;
        self.lineno = other.lineno;
    }

    pub fn advance(&mut self, string: &str) {
        self.index += string.len();
        self.lineno += count_newlines(string);
    }
}

struct Original<'a> {
    string: &'a str,
    lineno: usize,
}

struct Binding {
    key: String,
    value: Option<String>,
}

struct Reader {
    string: String,
    position: Position,
    mark: Position,
}

impl Reader {
    #[inline]
    pub fn new(string: String) -> Self {
        Self {
            string,
            position: Position::start(),
            mark: Position::start(),
        }
    }

    #[inline]
    pub fn has_next(&self) -> bool {
        self.position.index < self.string.len()
    }

    #[inline]
    pub fn set_mark(&mut self) {
        self.mark.set(&self.position);
    }

    #[inline]
    pub fn get_marked(&self) -> Original {
        Original {
            string: &self.string[self.mark.index..self.position.index],
            lineno: self.mark.lineno,
        }
    }

    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.string[self.position.index..].chars().next()
    }

    pub fn read_pattern(&mut self, pattern: fn(string: &str, index: usize) -> Option<Match>) -> Result<Match> {
        let Some(m) = pattern(&self.string, self.position.index) else {
            let line_start = self.string[..self.position.index].
                rfind(|ch| ch == '\n' || ch == '\r').
                map(|pos| pos + 1).
                unwrap_or(0);
            return Err(Error::syntax_error(self.position.lineno, 1 + self.position.index - line_start));
        };

        self.position.advance(&self.string[m.start_index..m.end_index]);

        Ok(m)
    }
}

struct Match<'a> {
    value: Option<&'a str>,
    start_index: usize,
    end_index: usize,
}

fn match_multiline_whitespace(string: &str, index: usize) -> Option<Match> {
    let Some(slice) = string.get(index..) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    };

    let Some(pos) = string.find(|ch: char| !ch.is_whitespace()) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: string.len(),
        });
    };

    Some(Match {
        value: None,
        start_index: index,
        end_index: index + pos,
    })
}

fn match_whitespace(string: &str, index: usize) -> Option<Match> {
    let Some(slice) = string.get(index..) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    };

    let Some(pos) = string.find(|ch: char| ch == '\r' || ch == '\n' || !ch.is_whitespace()) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: string.len(),
        });
    };

    Some(Match {
        value: None,
        start_index: index,
        end_index: index + pos,
    })
}

fn match_export(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_single_quoted_key(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_unquoted_key(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_equal_sign(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_single_quoted_value(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_double_quoted_value(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_unquoted_value(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_comment(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_end_of_line(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_rest_of_line(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_double_quoted_escapes(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn match_single_quoted_escapes(string: &str, index: usize) -> Option<Match> {
    unimplemented!()
}

fn count_newlines(mut src: &str) -> usize {
    let mut count = 0;

    while src.len() > 0 {
        let Some(index) = src.find(|ch| ch == '\n' || ch == '\r') else {
            break;
        };

        src = &src[index..];
        if src.starts_with("\r\n") {
            src = &src[2..];
        } else {
            src = &src[1..];
        }

        count += 1;
    }

    count
}

fn parse_key(reader: &mut Reader) -> Result<String> {
    unimplemented!()
}

fn parse_unquoted_value(reader: &mut Reader) -> String {
    unimplemented!()
}

fn parse_value(reader: &mut Reader) -> Result<Option<String>> {
    unimplemented!()
}

fn parse_binding(reader: &mut Reader) -> Result<Option<Binding>> {
    reader.set_mark();

    fn parse_binding_intern(reader: &mut Reader) -> Result<Option<Binding>> {
        reader.read_pattern(match_multiline_whitespace)?;

        if !reader.has_next() {
            return Ok(None);
        }

        reader.read_pattern(match_export)?;

        let key = parse_key(reader)?;

        reader.read_pattern(match_whitespace)?;

        let value = match reader.peek() {
            Some('=') => {
                reader.read_pattern(match_equal_sign)?;
                parse_value(reader)?
            },
            _ => None
        };

        reader.read_pattern(match_comment)?;
        reader.read_pattern(match_end_of_line)?;

        Ok(Some(Binding {
            key,
            value
        }))
    }

    let res = parse_binding_intern(reader);
    if res.is_err() {
        reader.read_pattern(match_rest_of_line)?;
    }

    res
}
