// trying to emulate: https://github.com/motdotla/dotenv/blob/8ab33066f90a20445d3c41e4fafba6c929c5e1a5/lib/main.js
use std::{io::BufRead, path::Path};

use crate::{Env, Error, Options, Result, DEBUG_PREFIX};

pub fn config_jsdotenv(reader: &mut dyn BufRead, env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();
    let mut buf = String::new();
    options.encoding.read_to_string(reader, &mut buf)?;

    fix_newlines(&mut buf);

    let mut parser = Parser {
        buf,
        line_start: 0,
        index: 0,
        lineno: 1,
    };

    while parser.index < parser.buf.len() {
        parser.skip_ws();

        if parser.buf[parser.index..].starts_with('#') {
            parser.index = find_line_end(&parser.buf, parser.index);
            continue;
        }

        if parser.index >= parser.buf.len() {
            break;
        }

        let mut key_start = parser.index;
        let mut key_end = find_vardef_end(&parser.buf, parser.index);

        parser.index = key_end;
        parser.skip_ws();

        if parser.buf[parser.index..].starts_with(is_vardef) && &parser.buf[key_start..key_end] == "export" {
            key_start = parser.index;
            key_end = find_vardef_end(&parser.buf, parser.index);
            parser.index = key_end;
            parser.skip_ws();
        }

        if key_start == key_end {
            let line_end = find_line_end(&parser.buf, parser.index);
            let column = key_end - find_line_start(&parser.buf, key_end) + 1;
            if options.debug {
                let line = &parser.buf[parser.line_start..line_end];
                if let Some(ch) = parser.buf[key_start..].chars().next() {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: expected variable name, found {:?}: {}",
                        parser.lineno, column, ch, line);
                } else {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: unexpected end of file: {}",
                        parser.lineno, column, line);
                }
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, column));
            }
            parser.index = line_end;
            continue;
        }

        let tail = &parser.buf[parser.index..];
        if !tail.starts_with(|ch| ch == '=' || ch == ':') {
            let line_end = find_line_end(&parser.buf, parser.index);
            let column = parser.index - parser.line_start + 1;
            let ch = tail.chars().next();

            if options.debug {
                let line = &parser.buf[parser.line_start..line_end];
                if let Some(ch) = ch {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: expected '=' or ':', found {:?}: {}",
                        parser.lineno, column, ch, line);
                } else {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: unexpected end of file: {}",
                        parser.lineno, column, line);
                }
            }

            if options.strict {
                return Err(Error::syntax_error(parser.lineno, column));
            }
            let ch = ch.unwrap_or('\0');
            if key_end < parser.line_start && is_vardef(ch) {
                // start parsing a variable definition from here
            } else {
                parser.index = line_end;
            }
            continue;
        } else if parser.index != key_end && tail.starts_with(':') {
            let line_end = find_line_end(&parser.buf, parser.index);
            let column = parser.index - parser.line_start + 1;

            if options.debug {
                let line = &parser.buf[parser.line_start..line_end];
                eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: there may be no space between the variable name and ':': {}",
                    parser.lineno, column, line);
            }

            if options.strict {
                return Err(Error::syntax_error(parser.lineno, column));
            }
            parser.index = line_end;
            continue;
        }

        parser.index += 1;
        parser.skip_ws_inline();

        let tail = &parser.buf[parser.index..];

        let value_start = parser.index;
        let mut value_end = value_start;
        let mut quote = false;
        if let Some(ch) = tail.chars().next() {
            if ch == '"' || ch == '\'' || ch == '`' {
                let lineno = parser.lineno;
                let line_start = parser.line_start;
                let index = parser.index;

                parser.index += 1;
                if parser.skip_to_quote_end(ch) {
                    quote = true;
                    value_end = parser.index + 1;
                    parser.index = value_end;
                    parser.skip_ws_inline();

                    let tail = &parser.buf[parser.index..];
                    if tail.starts_with('#') {
                        parser.index = find_line_end(&parser.buf, parser.index);
                    } else if !tail.is_empty() && !tail.starts_with('\n') {
                        // back out of parsing a quoted striung and fallback to normal string
                        quote = false;
                        parser.line_start = line_start;
                        parser.lineno = lineno;
                        parser.index = index;
                    }
                } else {
                    parser.line_start = line_start;
                    parser.lineno = lineno;
                    parser.index = index;
                }
            }
        }

        if !quote {
            value_end = find_value_end(&parser.buf, parser.index);
            parser.index = value_end;
        }

        let value_slice = &parser.buf[value_start..value_end];
        let quote = value_slice.chars().next().unwrap_or('\0');

        let mut value;
        if value_slice.len() > 1 && matches!(quote, '"' | '\'' | '`') && value_slice.ends_with(quote) {
            value = parser.buf[value_start + 1..value_end - 1].to_owned();
        } else {
            value = value_slice.trim_end_matches(|ch| matches!(ch, '\t' | '\x0B' | '\x0C' | ' ')).to_owned();
        }

        if quote == '"' {
            // yes, the original also applies unescape for a sorta
            // unquoted string that starts with a double quote
            value = unescape_double_quoted(&value);
        }

        options.set_var_cut_null(env, parser.buf[key_start..key_end].as_ref(), value.as_ref());

        parser.skip_ws_inline();
        let Some(ch) = parser.buf[parser.index..].chars().next() else {
            break;
        };

        if ch == '#' {
            parser.index = find_line_end(&parser.buf, parser.index);
        } else if ch != '\n' {
            let line_end = find_line_end(&parser.buf, parser.index);
            let column = parser.index - parser.line_start + 1;

            if options.debug {
                let line = &parser.buf[parser.line_start..line_end];
                eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: expected line end, found {:?}: {}",
                    parser.lineno, column, ch, line);
            }

            if options.strict {
                return Err(Error::syntax_error(parser.lineno, column));
            }
            parser.index = line_end;
        }
    }

    Ok(())
}

#[inline]
fn unescape_double_quoted(value: &str) -> String {
    value.replace("\\n", "\n").replace("\\r", "\r")
}

struct Parser {
    buf: String,
    line_start: usize,
    index: usize,
    lineno: usize,
}

impl Parser {
    fn skip_ws(&mut self) {
        for (index, ch) in self.buf[self.index..].char_indices() {
            if ch == '\n' {
                self.lineno += 1;
                self.line_start = self.index + index + 1;
            } else if !matches!(ch, '\t' | '\x0B' | '\x0C' | ' ') {
                self.index += index;
                return;
            }
        }
        self.index = self.buf.len();
    }

    fn skip_ws_inline(&mut self) {
        let len = self.buf.len();
        if self.index < len {
            self.index = self.buf[self.index..].
                find(|ch| !matches!(ch, '\t' | '\x0B' | '\x0C' | ' ')).
                map(|pos| pos + self.index).
                unwrap_or(len);
        }
    }

    fn skip_to_quote_end(&mut self, quote: char) -> bool {
        let mut iter = self.buf[self.index..].char_indices().peekable();
        while let Some((index, ch)) = iter.next() {
            if ch == '\n' {
                self.lineno += 1;
                self.line_start = self.index + index + 1;
            } else if ch == '\\' {
                let Some((_, ch)) = iter.peek() else {
                    break;
                };
                if *ch == quote {
                    iter.next();
                }
            } else if ch == quote {
                self.index += index;
                return true;
            }
        }

        false
    }
}

#[inline]
fn is_vardef(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-'
}

#[inline]
fn find_vardef_end(src: &str, index: usize) -> usize {
    let len = src.len();
    if index >= len {
        return len;
    }
    src[index..].find(|ch| !is_vardef(ch)).
        map(|pos| pos + index).
        unwrap_or(len)
}

#[inline]
fn find_value_end(src: &str, index: usize) -> usize {
    let len = src.len();
    if index >= len {
        return len;
    }
    src[index..].find(|ch| ch == '\n' || ch == '#').
        map(|pos| pos + index).
        unwrap_or(len)
}

#[inline]
fn find_line_end(src: &str, index: usize) -> usize {
    let len = src.len();
    if index >= len {
        return len;
    }
    src[index..].find('\n').
        map(|pos| pos + index).
        unwrap_or(len)
}

fn fix_newlines(buf: &mut String) {
    let mut index = 0;
    loop {
        let Some(cr_index) = buf[index..].find('\r').map(|pos| pos + index) else {
            break;
        };

        if buf[cr_index..].starts_with("\r\n") {
            buf.remove(cr_index);
        } else {
            buf.replace_range(cr_index..cr_index + 1, "\n");
        }

        index = cr_index + 1;
    }
}

#[inline]
fn find_line_start(src: &str, index: usize) -> usize {
    src[..index].rfind('\n').unwrap_or(0)
}
