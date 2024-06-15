// trying to emulate: https://github.com/bkeepers/dotenv/blob/27c80ed122f9bbe403033282e922d74ca717d518/lib/dotenv/parser.rb
use std::{ffi::OsStr, io::BufRead, path::Path};

use crate::{env::GetEnv, Env, Error, Options, Result, DEBUG_PREFIX};

pub fn config_rbdotenv(reader: &mut dyn BufRead, env: &mut dyn Env, parent: &dyn GetEnv, options: &Options<&Path>) -> Result<()> {
    let legacy_linebreak = if let Some(value) = parent.get("DOTENV_LINEBREAK_MODE".as_ref()) {
        let value: &OsStr = value.as_ref();
        value == "legacy"
    } else {
        false
    };
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

        let mut key_start = parser.index;
        let mut key_end = find_vardef_end(&parser.buf, parser.index);

        parser.index = key_end;
        parser.skip_ws_inline();

        let mut export = false;
        if parser.buf[parser.index..].starts_with(is_vardef) && &parser.buf[key_start..key_end] == "export" {
            export = true;
            key_start = parser.index;
            key_end = find_vardef_end(&parser.buf, parser.index);
            parser.index = key_end;
            parser.skip_ws_inline();
        }

        if key_start == key_end {
            let line_end = find_line_end(&parser.buf, parser.index);
            let column = key_end - parser.line_start + 1;
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
        if export && tail.starts_with(is_vardef) {
            // check that variables are set
            loop {
                let key = &parser.buf[key_start..key_end];
                if env.get(key.as_ref()).is_none() {
                    let line_end = find_line_end(&parser.buf, parser.index);
                    let column = key_end - parser.line_start + 1;
                    if options.debug {
                        let line = &parser.buf[parser.line_start..line_end];
                        eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: variable {key:?} is unset in line: {}",
                            parser.lineno, column, line);
                    }
                    if options.strict {
                        return Err(Error::syntax_error(parser.lineno, column));
                    }
                }

                parser.skip_ws_inline();

                if parser.buf[parser.index..].starts_with(|ch| ch == '#' || ch == '\n') {
                    parser.index = find_line_end(&parser.buf, parser.index);
                    break;
                }

                if parser.index >= parser.buf.len() {
                    break;
                }

                key_start = parser.index;
                key_end = find_vardef_end(&parser.buf, parser.index);
                parser.index = key_end;

                if key_start == key_end {
                    let line_end = find_line_end(&parser.buf, parser.index);
                    let column = key_end - parser.line_start + 1;
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
                    break;
                }
            }
            continue;
        } else if !tail.starts_with(|ch| ch == '=' || ch == ':') {
            let line_end = find_line_end(&parser.buf, parser.index);
            let column = parser.index - parser.line_start + 1;

            if options.debug {
                let line = &parser.buf[parser.line_start..line_end];
                if let Some(ch) = tail.chars().next() {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{}:{}: expected '=', found {:?}: {}",
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

        parser.index += 1;
        parser.skip_ws_inline();

        let tail = &parser.buf[parser.index..];

        let value_start = parser.index;
        let mut value_end = value_start;
        let mut quote = false;
        if let Some(ch) = tail.chars().next() {
            if ch == '"' || ch == '\'' {
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

        let value;
        if value_slice.len() > 1 && value_slice.starts_with('\'') && value_slice.ends_with('\'') {
            value = parser.buf[value_start + 1..value_end - 1].to_owned();
        } else if value_slice.len() > 1 && value_slice.starts_with('"') && value_slice.ends_with('"') {
            value = perform_substitutions(&unescape_double_quoted(&parser.buf[value_start + 1..value_end - 1], env, legacy_linebreak), env.as_get_env());
        } else {
            value = perform_substitutions(&unescape_single_unquoted(value_slice.trim_end_matches(|ch| matches!(ch, '\t' | '\x0B' | '\x0C' | ' '))), env.as_get_env());
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

fn perform_substitutions(mut src: &str, env: &dyn GetEnv) -> String {
    let mut buf = String::new();

    loop {
        let Some(index) = src.find(|ch| ch == '$' || ch == '\\') else {
            break;
        };

        buf.push_str(&src[..index]);
        src = &src[index..];

        if src.starts_with('\\') {
            src = &src[1..];
            let Some(ch) = src.chars().next() else {
                buf.push('\\');
                break;
            };
            if ch != '$' {
                buf.push('\\');
            }
            buf.push(ch);
            src = &src[ch.len_utf8()..];
        } else {
            let mut var_start = 1;
            if src.starts_with("${") {
                var_start += 1;
            }
            let var_end = find_substvar_end(src, var_start);
            if var_start == var_end {
                // ignore syntax error, like the original
                buf.push_str(&src[..var_end]);
                src = &src[var_end..];
            } else {
                let key = &src[var_start..var_end];
                if let Some(value) = env.get(key.as_ref()) {
                    buf.push_str(&value.to_string_lossy());
                }
                // yes, the { is independent to the } in the original!
                src = &src[var_end..];
                if src.starts_with('}') {
                    src = &src[1..];
                }
            }
        }
    }

    buf.push_str(src);

    buf
}

fn unescape_single_unquoted(mut value: &str) -> String {
    let mut buf = String::new();

    while !value.is_empty() {
        let Some(mut index) = value.find('\\') else {
            break;
        };

        let Some(ch) = value[index + 1..].chars().next() else {
            // backslash must be followed by something
            break;
        };

        buf.push_str(&value[..index]);

        match ch {
            '$' => buf.push_str("\\$"),
            _   => buf.push(ch),
        }

        index += 1 + ch.len_utf8();
        value = &value[index..];
    }

    buf.push_str(value);
    buf
}

fn unescape_double_quoted(mut value: &str, env: &dyn Env, parent_legacy_linebreak: bool) -> String {
    let legacy_linebreak = if let Some(value) = env.get("DOTENV_LINEBREAK_MODE".as_ref()) {
        let value: &OsStr = value.as_ref();
        value == "legacy"
    } else {
        parent_legacy_linebreak
    };

    let mut buf = String::new();

    while !value.is_empty() {
        let Some(mut index) = value.find('\\') else {
            break;
        };

        let Some(ch) = value[index + 1..].chars().next() else {
            // backslash must be followed by something
            break;
        };

        buf.push_str(&value[..index]);

        match ch {
            '$' => buf.push_str("\\$"),
            'n' => if legacy_linebreak { buf.push('\n'); } else { buf.push_str("\\n"); },
            'r' => if legacy_linebreak { buf.push('\r'); } else { buf.push_str("\\r"); },
            _   => buf.push(ch),
        }

        index += 1 + ch.len_utf8();
        value = &value[index..];
    }

    buf.push_str(value);
    buf
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
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
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
fn is_substvar(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[inline]
fn find_substvar_end(src: &str, index: usize) -> usize {
    let len = src.len();
    if index >= len {
        return len;
    }
    src[index..].find(|ch| !is_substvar(ch)).
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
