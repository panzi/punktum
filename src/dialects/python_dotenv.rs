// trying to emulate: https://github.com/theskumar/python-dotenv/blob/main/src/dotenv/parser.py
use std::{borrow::Cow, io::BufRead, path::Path};

use crate::{env::GetEnv, Env, Error, Options, Result, DEBUG_PREFIX};

pub fn config_python_dotenv(reader: &mut dyn BufRead, env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
    let mut string = String::new();
    options.encoding.read_to_string(reader, &mut string)?;
    let mut reader = Reader::new(string, options);

    while reader.has_next() {
        let binding = match reader.parse_binding() {
            Ok(None) => continue,
            Ok(Some(binding)) => binding,
            Err(err) => {
                if options.debug {
                    if let Some(loc) = err.location() {
                        eprintln!("{DEBUG_PREFIX}{}:{}:{}: invalid syntax", reader.path, loc.lineno(), loc.column());
                    } else {
                        let line_start = reader.string[..reader.position.index].rfind(|ch: char| ch == '\n' || ch == '\r').unwrap_or(0);
                        eprintln!("{DEBUG_PREFIX}{}:{}:{}: invalid syntax", reader.path, reader.position.lineno, reader.position.index - line_start + 1);
                    }
                }
                if options.strict {
                    return Err(err);
                }
                continue;
            }
        };

        let Some(key) = &binding.key else {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}: invalid syntax parsing key", reader.path, reader.mark.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(reader.mark.lineno, 1));
            }
            continue;
        };

        let value = if let Some(value) = &binding.value {
            value
        } else {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}: invalid syntax parsing value", reader.path, reader.mark.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(reader.mark.lineno, 1));
            }
            ""
        };

        // the original has interpolation as an option, but defaults to true
        let value = interpolate(value, env.as_get_env());
        options.set_var_cut_null(env, key, &value);
    }

    Ok(())
}

// see: https://github.com/theskumar/python-dotenv/blob/main/src/dotenv/variables.py
fn interpolate(mut src: &str, env: &dyn GetEnv) -> String {
    let mut buf = String::new();

    loop {
        let Some(index) = src.find('$') else {
            break;
        };

        buf.push_str(&src[..index]);
        src = &src[index + 1..];

        if src.starts_with('{') {
            src = &src[1..];
        }

        let index = src.find(|ch: char| ch == ':' || ch == '}').unwrap_or(src.len());
        let key = &src[..index];
        src = &src[index..];

        let default = if src.starts_with(":-") {
            src = &src[2..];
            let index = src.find('}').unwrap_or(src.len());
            let default = &src[..index];
            src = &src[index..];
            default
        } else {
            ""
        };

        if src.starts_with('}') {
            src = &src[1..];
        }

        if let Some(value) = env.get(key.as_ref()) {
            buf.push_str(value.to_string_lossy().as_ref());
        } else {
            buf.push_str(default);
        }
    }

    buf.push_str(src);

    buf
}

struct Position {
    index: usize,
    lineno: usize,
}

impl Position {
    #[inline]
    pub fn start() -> Self {
        Self { index: 0, lineno: 1 }
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

// struct Original<'a> {
//     string: &'a str,
//     lineno: usize,
// }

struct Binding {
    key: Option<String>,
    value: Option<String>,
}

struct Reader<'a> {
    string: String,
    position: Position,
    mark: Position,
    path: Cow<'a, str>,
}

impl<'a> Reader<'a> {
    #[inline]
    pub fn new(string: String, options: &Options<&'a Path>) -> Self {
        Self {
            string,
            position: Position::start(),
            mark: Position::start(),
            path: options.path.to_string_lossy()
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

    // #[inline]
    // pub fn get_marked(&self) -> Original {
    //     Original {
    //         string: &self.string[self.mark.index..self.position.index],
    //         lineno: self.mark.lineno,
    //     }
    // }

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

    fn parse_key(&mut self) -> Result<Option<String>> {
        let Some(ch) = self.peek() else {
            return Ok(None);
        };

        if ch == '#' {
            return Ok(None);
        }

        if ch == '\'' {
            let res = self.read_pattern(match_single_quoted_key)?;
            return Ok(res.value.map(str::to_string));
        }

        let res = self.read_pattern(match_unquoted_key)?;
        Ok(res.value.map(str::to_string))
    }

    fn parse_unquoted_value(&mut self) -> Result<String> {
        let res = self.read_pattern(match_unquoted_value)?;
        let mut value = res.value.unwrap_or("");
        if let Some(index) = value.find('#') {
            value = &value[..index];
        }
        value = value.trim_end();
        Ok(value.to_owned())
    }

    fn parse_value(&mut self) -> Result<String> {
        let Some(ch) = self.peek() else {
            return Ok(String::new());
        };
        if ch == '\'' {
            let res = self.read_pattern(match_single_quoted_value)?;
            let value = res.value.unwrap_or("");
            Ok(decode_single_quote_escapes(value))
        } else if ch == '"' {
            let res = self.read_pattern(match_double_quoted_value)?;
            let value = res.value.unwrap_or("");
            Ok(decode_double_quote_escapes(value))
        } else if ch == '\n' || ch == '\r' {
            Ok(String::new())
        } else {
            Ok(self.parse_unquoted_value()?)
        }
    }

    fn parse_binding(&mut self) -> Result<Option<Binding>> {
        self.set_mark();

        fn parse_binding_intern(reader: &mut Reader) -> Result<Option<Binding>> {
            reader.read_pattern(match_multiline_whitespace)?;

            if !reader.has_next() {
                return Ok(None);
            }

            reader.read_pattern(match_export)?;

            let key = reader.parse_key()?;

            reader.read_pattern(match_whitespace)?;

            let value = match reader.peek() {
                Some('=') => {
                    reader.read_pattern(match_equal_sign)?;
                    Some(reader.parse_value()?)
                },
                _ => None
            };

            reader.read_pattern(match_comment)?;
            reader.read_pattern(match_end_of_line)?;

            Ok(Some(Binding {
                key,
                value,
            }))
        }

        let res = parse_binding_intern(self);
        if res.is_err() {
            self.read_pattern(match_rest_of_line)?;
        }

        res
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

    let Some(pos) = slice.find(|ch: char| !ch.is_whitespace()) else {
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

#[inline]
fn is_inline_whitespace(ch: char) -> bool {
    ch != '\r' && ch != '\n' && ch.is_whitespace()
}

fn match_whitespace(string: &str, index: usize) -> Option<Match> {
    let Some(slice) = string.get(index..) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    };

    let Some(pos) = slice.find(|ch: char| !is_inline_whitespace(ch)) else {
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
    let slice = &string[index..];

    let Some(slice) = slice.strip_prefix("export") else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index
        });
    };

    if !slice.starts_with(is_inline_whitespace) {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index
        });
    }

    let pos = slice.find(|ch: char| !is_inline_whitespace(ch)).unwrap_or(slice.len());

    if pos == 0 {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index
        });
    }

    Some(Match {
        value: None,
        start_index: index,
        end_index: index + "export".len() + pos
    })
}

fn match_single_quoted_key(string: &str, index: usize) -> Option<Match> {
    let slice = &string[index..];

    if !slice.starts_with('\'') {
        return None;
    }

    let slice = &string[1..];
    let pos = slice.find('\'')?;

    if pos == 0 {
        return None;
    }

    Some(Match {
        value: Some(&slice[..pos]),
        start_index: index,
        end_index: index + pos + 2
    })
}

fn match_unquoted_key(string: &str, index: usize) -> Option<Match> {
    let slice = &string[index..];

    let pos = slice.find(|ch: char| ch == '=' || ch == '#' || ch.is_whitespace()).unwrap_or(slice.len());

    if pos == 0 {
        return None;
    }

    Some(Match {
        value: Some(&slice[..pos]),
        start_index: index,
        end_index: index + pos
    })
}

fn match_equal_sign(string: &str, index: usize) -> Option<Match> {
    let slice = &string[index..];

    if !slice.starts_with('=') {
        return None;
    }

    let slice = &string[1..];
    let pos = slice.find(|ch: char| !is_inline_whitespace(ch)).unwrap_or(slice.len());

    Some(Match {
        value: None,
        start_index: index,
        end_index: index + 1 + pos,
    })
}

#[inline]
fn match_single_quoted_value(string: &str, index: usize) -> Option<Match> {
    match_quoted_value(string, index, '\'')
}

#[inline]
fn match_double_quoted_value(string: &str, index: usize) -> Option<Match> {
    match_quoted_value(string, index, '"')
}

fn match_quoted_value(string: &str, index: usize, quote: char) -> Option<Match> {
    // emulating the regex r"'((?:\\'|[^'])*)'" including backtracking so it
    // also matches r"'\'"
    let slice = &string[index..];

    if !slice.starts_with(quote) {
        return None;
    }

    let quote_len = quote.len_utf8();
    let slice = &slice[quote_len..];
    let mut end_index = slice.find(quote)?;

    while slice[..end_index].ends_with('\\') {
        let Some(pos) = slice[end_index + quote_len..].find(quote) else {
            break;
        };
        end_index += pos + quote_len;
    }

    Some(Match {
        value: Some(&slice[..end_index]),
        start_index: index,
        end_index: index + quote_len + end_index + quote_len,
    })
}

fn match_unquoted_value(string: &str, index: usize) -> Option<Match> {
    let slice = &string[index..];

    let pos = slice.find(|ch: char| ch == '\n' || ch == '\r').unwrap_or(slice.len());

    if pos == 0 {
        return None;
    }

    Some(Match {
        value: Some(&slice[..pos]),
        start_index: index,
        end_index: index + pos
    })
}

fn match_comment(string: &str, index: usize) -> Option<Match> {
    let Some(slice) = string.get(index..) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    };

    let Some(hash_index) = slice.find(|ch: char| !is_inline_whitespace(ch)) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    };

    let slice = &slice[hash_index..];
    if !slice.starts_with('#') {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    }

    let end_index = if let Some(pos) = slice.find(|ch: char| ch == '\n' || ch == '\r') {
        pos
    } else {
        slice.len()
    };

    Some(Match {
        value: None,
        start_index: index,
        end_index: index + hash_index + end_index,
    })
}

fn match_end_of_line(string: &str, index: usize) -> Option<Match> {
    let Some(slice) = string.get(index..) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    };

    let Some(pos) = slice.find(|ch: char| !is_inline_whitespace(ch)) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: string.len(),
        });
    };

    let slice = &slice[pos..];
    let end_index = index + pos + if slice.starts_with("\r\n") {
        2
    } else if slice.starts_with(|ch: char| ch == '\n' || ch == '\r') {
        1
    } else {
        return None;
    };

    Some(Match {
        value: None,
        start_index: index,
        end_index
    })
}

fn match_rest_of_line(string: &str, index: usize) -> Option<Match> {
    let Some(slice) = string.get(index..) else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: index,
        });
    };

    let Some(pos) = slice.find(|ch: char| ch == '\n' || ch == '\r') else {
        return Some(Match {
            value: None,
            start_index: index,
            end_index: string.len(),
        });
    };

    let slice = &slice[pos..];
    let end_index = index + pos + if slice.starts_with("\r\n") {
        2
    } else {
        1
    };

    Some(Match {
        value: None,
        start_index: index,
        end_index
    })
}

fn count_newlines(mut src: &str) -> usize {
    let mut count = 0;

    while !src.is_empty() {
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

fn decode_single_quote_escapes(mut value: &str) -> String {
    let mut buf = String::with_capacity(value.len());

    loop {
        let Some(index) = value.find('\\') else {
            break;
        };

        buf.push_str(&value[..index]);

        value = &value[index + 1..];
        if value.starts_with('\\') {
            buf.push('\\');
            value = &value[1..];
        } else if value.starts_with('\'') {
            buf.push('\'');
            value = &value[1..];
        } else {
            buf.push('\\');
        }
    }
    buf.push_str(value);

    buf.shrink_to_fit();
    buf
}

fn decode_double_quote_escapes(mut value: &str) -> String {
    let mut buf = String::with_capacity(value.len());

    loop {
        let Some(index) = value.find('\\') else {
            break;
        };

        buf.push_str(&value[..index]);

        value = &value[index + 1..];
        let Some(ch) = value.chars().next() else {
            buf.push('\\');
            break;
        };

        match ch {
            '\n' => {
                value = &value[1..];
            }
            '\\' | '\'' | '"' => {
                buf.push(ch);
                value = &value[1..];
            }
            'a' => {
                buf.push('\x07');
                value = &value[1..];
            }
            'b' => {
                buf.push('\x08');
                value = &value[1..];
            }
            'f' => {
                buf.push('\x0c');
                value = &value[1..];
            }
            'n' => {
                buf.push('\n');
                value = &value[1..];
            }
            'r' => {
                buf.push('\r');
                value = &value[1..];
            }
            't' => {
                buf.push('\t');
                value = &value[1..];
            }
            'v' => {
                buf.push('\x0b');
                value = &value[1..];
            }
            '0'|'1'|'2'|'3'|'4'|'5'|'6'|'7' => {
                let mut end_index = 1;
                if value.len() > 1 && value[1..].starts_with(|ch| matches!(ch, '0'|'1'|'2'|'3'|'4'|'5'|'6'|'7')) {
                    end_index += 1;
                    if value.len() > 2 && value[2..].starts_with(|ch| matches!(ch, '0'|'1'|'2'|'3'|'4'|'5'|'6'|'7')) {
                        end_index += 1;
                    }
                }

                let arg = &value[..end_index];
                let Ok(ch) = u8::from_str_radix(arg, 8) else {
                    buf.push('\\');
                    continue;
                };
                buf.push(ch as char);
                value = &value[end_index..];
            }
            'x' => {
                if value.len() < 3 {
                    buf.push('\\');
                    continue;
                }

                let arg = &value[1..3];
                let Ok(ch) = u8::from_str_radix(arg, 16) else {
                    buf.push('\\');
                    continue;
                };
                buf.push(ch as char);
                value = &value[3..];
            }
            'u' => {
                if value.len() < 5 {
                    buf.push('\\');
                    continue;
                }

                let arg = &value[1..5];
                let Ok(ch) = u16::from_str_radix(arg, 16) else {
                    buf.push('\\');
                    continue;
                };
                let Some(ch) = char::from_u32(ch.into()) else {
                    // XXX: this should probably throw in the original?
                    buf.push('\\');
                    continue;
                };
                buf.push(ch);
                value = &value[5..];
            }
            'U' => {
                if value.len() < 7 {
                    buf.push('\\');
                    continue;
                }

                let arg = &value[1..7];
                let Ok(ch) = u32::from_str_radix(arg, 16) else {
                    buf.push('\\');
                    continue;
                };
                let Some(ch) = char::from_u32(ch) else {
                    buf.push('\\');
                    continue;
                };
                buf.push(ch);
                value = &value[7..];
            }
            _ => {
                buf.push('\\');
                continue;
            }
        }
    }
    buf.push_str(value);

    buf.shrink_to_fit();
    buf
}
