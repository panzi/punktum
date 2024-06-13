// trying to emulate: https://github.com/bkeepers/dotenv/blob/27c80ed122f9bbe403033282e922d74ca717d518/lib/dotenv/parser.rb
use std::{io::BufRead, path::Path};

use crate::{env::GetEnv, Env, Error, Options, Result, DEBUG_PREFIX};

pub fn config_rbdotenv(reader: &mut dyn BufRead, env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
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
        // yes, the original skips lines here!
        parser.skip_ws();

        let mut export = false;
        if parser.buf[parser.index..].starts_with(is_vardef) && &parser.buf[key_start..key_end] == "export" {
            export = true;
            key_start = parser.index;
            key_end = find_vardef_end(&parser.buf, parser.index);
            // yes, the original skips lines here!
            parser.skip_ws();
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

        parser.index += 1;
        parser.skip_ws_inline();

        let tail = &parser.buf[parser.index..];

        let value_start = parser.index;
        let mut value_end = value_start;
        let quote;
        if let Some(ch) = tail.chars().next() {
            if ch == '"' || ch == '\'' {
                parser.index += 1;
                parser.skip_to_quote_end(ch);
                quote = ch;
                value_end = parser.index;
            } else {
                quote = '\0';
            }
        } else {
            quote = '\0';
        }

        let value;
        if quote == '\0' {
            value_end = find_value_end(&parser.buf, parser.index);
            parser.index = value_end;
            value = perform_substitutions(&unescape_unquoted(&parser.buf[value_start..value_end]), env.as_get_env());
        } else if quote == '\'' {
            value = perform_substitutions(&parser.buf[value_start + 1..value_end - 1], env.as_get_env());
        } else {
            value = perform_substitutions(&unescape_double_quoted(&parser.buf[value_start + 1..value_end - 1]), env.as_get_env());
        }

        options.set_var_cut_null(env, parser.buf[key_start..key_end].as_ref(), value.as_ref());
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
                break;
            };
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

fn unescape_unquoted(mut value: &str) -> String {
    let mut buf = String::new();

    while !value.is_empty() {
        let Some(mut index) = value.find('\\') else {
            break;
        };

        buf.push_str(&value[..index]);

        if value[index..].starts_with("\\$") {
            buf.push_str("\\$");
            index += 2;
        } else {
            index += 1;
        }

        value = &value[index..];
    }

    buf.push_str(value);
    buf
}

fn unescape_double_quoted(mut value: &str) -> String {
    let mut buf = String::new();

    while !value.is_empty() {
        let Some(index) = value.find('\\') else {
            break;
        };

        buf.push_str(&value[..index]);
        value = &value[index + 1..];

        if value.starts_with('$') {
            buf.push_str("\\$");
            value = &value[1..];
        } else if value.starts_with('n') {
            buf.push('\n');
            value = &value[1..];
        } else if value.starts_with('r') {
            buf.push('\r');
            value = &value[1..];
        }
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

    fn skip_to_quote_end(&mut self, quote: char) {
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
                return;
            }
        }
        self.index = self.buf.len();
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
