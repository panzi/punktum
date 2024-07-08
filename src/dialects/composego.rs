use std::{borrow::Cow, io::BufRead, num::NonZeroU8, path::Path};

use crate::{env::GetEnv, Env, Error, Options, Result, DEBUG_PREFIX};

// trying to be compatible to: https://github.com/compose-spec/compose-go/blob/main/dotenv/parser.go
pub fn config_composego(reader: &mut dyn BufRead, env: &mut dyn Env, parent: &dyn GetEnv, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();

    let mut src = String::new();
    options.encoding.read_to_string(reader, &mut src)?;

    // strip byte order mark
    let src = src.strip_prefix('\u{FEFF}').unwrap_or(&src);

    // The original doesn't change the line endings?
    // TODO: Test this bahavior!
    // let src = src.replace("\r\n", "\n");

    let mut cutset = src;
    let mut parser = Parser {
        lineno: 1,
        path: path_str,
        debug: options.debug,
        strict: options.strict,
    };

    loop {
        let Some(next_cutset) = parser.get_statement_start(cutset) else {
            break;
        };
        cutset = next_cutset;
        if cutset.is_empty() {
            break;
        }

        let (key, left, inherited) = match parser.locate_key_name(cutset) {
            Err(err) => {
                if options.strict {
                    return Err(err);
                }
                cutset = skip_to_line_end(cutset);
                continue;
            },
            Ok(res) => res
        };

        if key.contains(' ') {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}: key cannot contain a space: {key:?}", &parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, 1));
            }
            cutset = skip_to_line_end(cutset);
            continue;
        }

        let raw_key = key;
        let key = key.split('\0').next().unwrap();
        if key.len() != raw_key.len() {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}: key contains null byte: {key:?}", &parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, 1));
            }
        }

        if inherited {
            if let Some(value) = parent.get(key.as_ref()) {
                options.set_var(env, key.as_ref(), &value);
            }
            cutset = left;
            continue;
        }
        let (value, left) = match parser.extract_var_value(left, env.as_get_env()) {
            Err(err) => {
                if options.strict {
                    return Err(err);
                }
                cutset = skip_to_line_end(cutset);
                continue;
            },
            Ok(res) => res
        };
        let raw_value = &value;
        let value = value.split('\0').next().unwrap();
        if value.len() != raw_value.len() {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}: value of key {key:?} contains null byte: {value:?}", &parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, 1));
            }
        }

        options.set_var(env, key.as_ref(), value.as_ref());

        cutset = left;
    }

    Ok(())
}

struct Parser<'a> {
    lineno: usize,
    path: Cow<'a, str>,
    debug: bool,
    strict: bool,
}

impl<'a> Parser<'a> {
    fn get_statement_start<'b>(&mut self, mut src: &'b str) -> Option<&'b str> {
        loop {
            let pos = self.index_of_non_space_char(src)?;

            src = &src[pos..];
            if !src.starts_with('#') {
                return Some(src);
            }

            let pos = src.find('\n')?;

            src = &src[pos..]
        }
    }

    #[inline]
    fn index_of_non_space_char(&mut self, src: &str) -> Option<usize> {
        src.find(|ch: char| {
            if ch == '\n' {
                self.lineno += 1;
            }
            !ch.is_whitespace()
        })
    }

    fn trim_export<'b>(&self, src: &'b str) -> &'b str {
        if !src.starts_with("export") {
            return src;
        }

        let left = &src[6..];
        if !left.starts_with(is_space) {
            return src;
        }

        let Some(index) = left.find(|ch: char| !is_space(ch)) else {
            return "";
        };

        &left[index..]
    }

    fn locate_key_name<'b>(&mut self, mut src: &'b str) -> Result<(&'b str, &'b str, bool)> {
        let mut key = "";
        let mut inherited = false;
        src = self.trim_export(src);

        let mut offset = 0;
        for (index, rune) in src.char_indices() {
            if is_space(rune) {
                continue;
            }

            match rune {
                '='|':'|'\n' => {
                    key = &src[..index];
                    offset = index + 1;
                    inherited = rune == '\n';
                    break;
                }
                '_'|'.'|'-'|'['|']' => {}
                _ => {
                    if rune.is_alphanumeric() {
                        continue;
                    }

                    if self.debug {
                        let newline = src.find('\n').unwrap_or(src.len());
                        eprintln!("{DEBUG_PREFIX}{}:{}: unexpected character {:?} in variable name {:?}",
                            &self.path, self.lineno, rune, &src[..newline]
                        );
                    }
    
                    return Err(Error::syntax_error(self.lineno, 1));
                }
            }
        }

        if src.is_empty() {
            if self.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}: zero length string",
                    &self.path, self.lineno
                );
            }

            return Err(Error::syntax_error(self.lineno, 1));
        }

        key = key.trim_end();
        let cutset = src[offset..].trim_start_matches(is_space);

        if inherited {
            self.lineno += 1;
        }

        Ok((key, cutset, inherited))
    }

    fn extract_var_value<'b>(&mut self, src: &'b str, env: &dyn GetEnv) -> Result<(String, &'b str)> {
        let Some(quote) = has_quote_prefix(src) else {
            let index = src.find('\n').unwrap_or(src.len());
            let mut value = &src[..index];
            let rest = if index < src.len() {
                self.lineno += 1;
                &src[index + 1..]
            } else {
                &src[index..]
            };

            if let Some(index) = value.find(" #") {
                value = &value[..index];
            }
            value = value.trim_end();
            let res = self.expand_variables(value, env)?;
            return Ok((res, rest));
        };

        let mut prev_char_is_esc = false;

        let quote = quote.get() as char;
        let mut value = String::new();
        let quoted_start = src;
        let src = &src[1..];
        for (index, ch) in src.char_indices() {
            if ch == '\n' {
                self.lineno += 1;
            }
            if ch != quote {
                if !prev_char_is_esc && ch == '\\' {
                    prev_char_is_esc = true;
                    continue;
                }
                if prev_char_is_esc {
                    prev_char_is_esc = false;
                    value.push('\\');
                }
                value.push(ch);
                continue;
            }

            if prev_char_is_esc {
                prev_char_is_esc = false;
                value.push(ch);
                continue;
            }

            if quote == '"' {
                let res = self.expand_variables(&expand_escapes(&value), env)?;
                return Ok((res, &src[index + ch.len_utf8()..]));
            }

            return Ok((value, &src[index + ch.len_utf8()..]));
        }

        if self.debug {
            eprintln!("{DEBUG_PREFIX}{}:{}: unterminated quoted value: {}",
                &self.path, self.lineno, &quoted_start
            );
        }

        Err(Error::syntax_error(self.lineno, 1))
    }

    // see: https://github.com/compose-spec/compose-go/blob/e1496cd905b20b799fa3acecefed8056338961a2/template/template.go
    fn expand_variables(&self, mut src: &str, env: &dyn GetEnv) -> Result<String> {
        let mut buf = String::new();

        while !src.is_empty() {
            let Some(index) = src.find('$') else {
                buf.push_str(src);
                break;
            };

            buf.push_str(&src[..index]);
            let subst_start = &src[index..];
            src = &subst_start[1..];

            let Some(ch) = src.chars().next() else {
                if self.debug {
                    eprintln!("{DEBUG_PREFIX}{}:{}: single $ encounterd",
                        &self.path, self.lineno
                    );
                }
                buf.push('$');
                break;
            };

            match ch {
                '$' => {
                    buf.push('$');
                    src = &src[1..];
                }
                '{' => {
                    src = &src[1..];
                    let index = find_var_subst_end(src);
                    if index == 0 || !src.starts_with(is_var_subst_start) {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: substitution syntax truncated: {}",
                                &self.path, self.lineno, &subst_start[..2]
                            );
                        }
                        if self.strict {
                            return Err(Error::syntax_error(self.lineno, 1));
                        }
                        buf.push_str("${");
                        continue;
                    }

                    let name = &src[..index].split('\0').next().unwrap();
                    src = &src[index..];
                    let value = env.get(name.as_ref());
                    let var_end_index = if src.starts_with(|ch| ch == ':' || ch == '?' ||ch == '+' || ch == '-') {
                        let var_end_index = find_braced_subst_end(src);
                        if var_end_index >= src.len() {
                            if self.debug {
                                eprintln!("{DEBUG_PREFIX}{}:{}: expected: \"}}\", actual: {src:?}",
                                    &self.path, self.lineno
                                );
                            }
                            if self.strict {
                                return Err(Error::syntax_error(self.lineno, 1));
                            }
                            buf.push_str("${");
                            buf.push_str(name);
                            continue;
                        }
                        var_end_index
                    } else if src.starts_with('}') {
                        0
                    } else {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: expected: \"}}\", actual: {src:?}",
                                &self.path, self.lineno
                            );
                        }
                        if self.strict {
                            return Err(Error::syntax_error(self.lineno, 1));
                        }
                        buf.push_str("${");
                        buf.push_str(name);
                        continue;
                    };

                    if src.starts_with(":?") {
                        // required error when empty or unset
                        let message = &src[2..var_end_index];
                        src = &src[var_end_index..];
                        if let Some(value) = value {
                            if value.is_empty() {
                                if self.debug {
                                    let message = self.expand_variables(message, env)?;
                                    if message.is_empty() {
                                        eprintln!("{DEBUG_PREFIX}{}:{}: variable ${} may not be empty",
                                            &self.path, self.lineno, name
                                        );
                                    } else {
                                        eprintln!("{DEBUG_PREFIX}{}:{}: {}",
                                            &self.path, self.lineno, message
                                        );
                                    }
                                }
                                return Err(Error::syntax_error(self.lineno, 1));
                            }
                            buf.push_str(value.to_string_lossy().as_ref());
                        } else {
                            if self.debug {
                                let message = self.expand_variables(message, env)?;
                                if message.is_empty() {
                                    eprintln!("{DEBUG_PREFIX}{}:{}: variable ${} may not be unset",
                                        &self.path, self.lineno, name
                                    );
                                } else {
                                    eprintln!("{DEBUG_PREFIX}{}:{}: {}",
                                        &self.path, self.lineno, message
                                    );
                                }
                            }
                            return Err(Error::syntax_error(self.lineno, 1));
                        }
                    } else if src.starts_with('?') {
                        // required error when unset
                        let message = &src[1..var_end_index];
                        src = &src[var_end_index..];
                        if let Some(value) = value {
                            buf.push_str(value.to_string_lossy().as_ref());
                        } else {
                            if self.debug {
                                let message = self.expand_variables(message, env)?;
                                if message.is_empty() {
                                    eprintln!("{DEBUG_PREFIX}{}:{}: variable ${} may not be unset",
                                        &self.path, self.lineno, name
                                    );
                                } else {
                                    eprintln!("{DEBUG_PREFIX}{}:{}: {}",
                                        &self.path, self.lineno, message
                                    );
                                }
                            }
                            return Err(Error::syntax_error(self.lineno, 1));
                        }
                    } else if src.starts_with(":-") {
                        // default when empty or unset
                        let default = &src[2..var_end_index];
                        src = &src[var_end_index..];
                        if let Some(value) = value {
                            if value.is_empty() {
                                let default = self.expand_variables(default, env)?;
                                buf.push_str(&default);
                            } else {
                                buf.push_str(value.to_string_lossy().as_ref());
                            }
                        } else {
                            let default = self.expand_variables(default, env)?;
                            buf.push_str(&default);
                        }
                    } else if src.starts_with('-') {
                        // default when unset
                        let default = &src[1..var_end_index];
                        src = &src[var_end_index..];
                        if let Some(value) = value {
                            buf.push_str(value.to_string_lossy().as_ref());
                        } else {
                            let default = self.expand_variables(default, env)?;
                            buf.push_str(&default);
                        }
                    } else if src.starts_with(":+") {
                        // default when not empty
                        let default = &src[2..var_end_index];
                        src = &src[var_end_index..];
                        if let Some(value) = value {
                            if !value.is_empty() {
                                let default = self.expand_variables(default, env)?;
                                buf.push_str(&default);
                            }
                        }
                    } else if src.starts_with('+') {
                        // default when set
                        let default = &src[1..var_end_index];
                        src = &src[var_end_index..];
                        if value.is_some() {
                            let default = self.expand_variables(default, env)?;
                            buf.push_str(&default);
                        }
                    } else if let Some(value) = value {
                        buf.push_str(value.to_string_lossy().as_ref());
                    }

                    if !src.starts_with('}') {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: expected: \"}}\", actual: {src:?}",
                                &self.path, self.lineno
                            );
                        }
                        if self.strict {
                            return Err(Error::syntax_error(self.lineno, 1));
                        }
                    } else {
                        src = &src[1..];
                    }
                }
                _ => {
                    if !src.starts_with(is_var_subst_start) {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: ignored substitution syntax error: {:?}",
                                &self.path, self.lineno, &subst_start[..1 + ch.len_utf8()]
                            );
                        }
                        // seems to be ignored by the orginal
                        buf.push('$');
                    } else {
                        let index = find_var_subst_end(src);
                        if index == 0 {
                            if self.debug {
                                eprintln!("{DEBUG_PREFIX}{}:{}: substitution syntax error: {:?}",
                                    &self.path, self.lineno, &subst_start[..1 + ch.len_utf8()]
                                );
                            }
                            if self.strict {
                                return Err(Error::syntax_error(self.lineno, 1));
                            }
                            buf.push('$');
                        } else {
                            let name = &src[..index].split('\0').next().unwrap();
                            src = &src[index..];

                            if let Some(value) = env.get(name.as_ref()) {
                                buf.push_str(value.to_string_lossy().as_ref());
                            }
                        }
                    }
                }
            }
        }

        Ok(buf)
    }
}

#[inline]
fn is_var_subst_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

#[inline]
fn find_var_subst_end(src: &str) -> usize {
    src.find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_').unwrap_or(src.len())
}

fn expand_escapes(mut src: &str) -> String {
    let mut buf = String::new();

    while !src.is_empty() {
        let Some(index) = src.find('\\') else {
            break;
        };

        buf.push_str(&src[..index]);
        src = &src[index + 1..];

        let Some(ch) = src.chars().next() else {
            buf.push('\\');
            break;
        };

        src = &src[ch.len_utf8()..];

        match ch {
            'a' => buf.push('\x07'),
            'b' => buf.push('\x08'),
            // 'c' => {} // XXX: I think this is a mistake in the original? Not sure.
            'f' => buf.push('\x0C'),
            'n' => buf.push('\n'),
            'r' => buf.push('\r'),
            't' => buf.push('\t'),
            'v' => buf.push('\x0B'),
            '$' => buf.push_str("$$"),
            '"' => buf.push('"'),
            '\\' => buf.push('\\'),
            '0' => {
                // The original replaces any "\0" prefix with just "\", but
                // still uses that changed string in the fallback if unescaping
                // somehow fails. And if fails in cases where it shouldn't,
                // because of the buggy regular expression.
                if let Some(oct) = src.get(..3) {
                    if let Ok(byte) = u8::from_str_radix(oct, 8) {
                        buf.push(byte as char);
                        src = &src[3..];
                    } else {
                        buf.push('\\');
                    }
                } else {
                    buf.push('\\');
                }
            }
            _ => {
                buf.push('\\');
                buf.push(ch);
            }
        }
    }

    buf.push_str(src);
    buf
}

fn has_quote_prefix(src: &str) -> Option<NonZeroU8> {
    if src.starts_with('"') {
        return NonZeroU8::new(b'"');
    } else if src.starts_with('\'') {
        return NonZeroU8::new(b'\'');
    }
    None
}

#[inline]
fn is_space(ch: char) -> bool {
    matches!(ch, '\t' | '\x0B' | '\x0C' | '\r' | ' ' | '\u{85}' | '\u{A0}')
}

// I'm not sure if the original can even handle nested variable substitutions like that.
// Correctly emulates broken behavior of these:
//
//      FOO="${BAR:-{baz}
//      bla}"
//
//      FOO="${BAR:-{{}
//      }"
//
// And these (as in gives an error for these):
//
//      FOO="${BAR:-
//      }"
//
//      FOO="${BAR:-${BAZ}
//      }"

fn find_braced_subst_end(src: &str) -> usize {
    // Trying to emulated weird regular expression behavior.
    // They use a greedy regular .* expression for the defaul/message part,
    // but then it eats too much if there is another variable substitution
    // in the same string, so they use this logic to find the actual balanced
    // end brace and handle the part after it recursively.
    // While I don't need a hack for the wrong use of a regular expression I
    // have to emulate the behavior here.
    // See: https://github.com/compose-spec/compose-go/blob/9d0d133e13d0955e27520c6317d08822b7c5de5f/template/template.go#L252
    let mut nesting = 0;
    let mut prev_closing = src.len();
    for (index, ch) in src.char_indices() {
        if ch == '}' {
            if nesting == 0 {
                return index;
            }
            nesting -= 1;
            prev_closing = index;
        } else if ch == '{' {
            nesting += 1;
        } else if ch == '\n' {
            break;
        }
    }
    prev_closing
}

#[inline]
fn skip_to_line_end(src: &str) -> &str {
    let Some(index) = src.find('\n') else {
        return "";
    };
    &src[index..]
}
