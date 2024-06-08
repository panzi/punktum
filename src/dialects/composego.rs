use std::{borrow::Cow, fs::File, io::BufReader, num::NonZeroU8, path::Path};

use crate::{env::GetEnv, Env, Error, ErrorKind, Options, Result, DEBUG_PREFIX};

// trying to be compatible to: https://github.com/compose-spec/compose-go/blob/main/dotenv/parser.go
// maybe also implement this? https://github.com/joho/godotenv/blob/v1.5.1/parser.go
pub fn config_composego(env: &mut dyn Env, parent: &dyn GetEnv, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();

    let src = match File::open(options.path) {
        Err(err) => {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}: {err}");
            }
            if options.strict {
                return Err(Error::with_cause(ErrorKind::IOError, err));
            }
            return Ok(());
        }
        Ok(file) => {
            let mut src = String::new();
            options.encoding.read_to_string(&mut BufReader::new(file), &mut src)?;
            src
        }
    };

    let src = src.replace("\r\n", "\n");
    let mut cutset = &src[..];
    let mut parser = Parser {
        lineno: 1,
        path: path_str,
        debug: options.debug,
    };

    loop {
        let Some(next_cutset) = parser.get_statement_start(cutset) else {
            break;
        };
        cutset = next_cutset;
        if cutset.is_empty() {
            break;
        }

        let (key, left, inherited) = parser.locate_key_name(cutset)?;

        if key.contains(' ') {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}: key cannot contain a space", &parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, 1));
            }
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
        let (value, left) = parser.extract_var_value(left, env)?;
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

    fn locate_key_name<'b>(&self, mut src: &'b str) -> Result<(&'b str, &'b str, bool)> {
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
                    if rune.is_ascii_alphanumeric() {
                        continue;
                    }

                    if self.debug {
                        let newline = src.find('\n').unwrap_or(src.len());
                        eprintln!("{DEBUG_PREFIX}{}:{}: unexpected character {} in variable name {:?}",
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

        Ok((key, cutset, inherited))
    }

    fn extract_var_value<'b>(&mut self, src: &'b str, env: &mut dyn Env) -> Result<(String, &'b str)> {
        let Some(quote) = has_quote_prefix(src) else {
            let index = src.find('\n').unwrap_or(src.len());
            let (mut value, rest) = src.split_at(index);
            self.lineno += 1;

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

        let val_end_index = src.find('\n').unwrap_or(src.len());
        if self.debug {
            eprintln!("{DEBUG_PREFIX}{}:{}: unterminated quoted value {}",
                &self.path, self.lineno, &src[..val_end_index]
            );
        }

        Err(Error::syntax_error(self.lineno, 1))
    }

    // see: https://github.com/compose-spec/compose-go/blob/e1496cd905b20b799fa3acecefed8056338961a2/template/template.go
    fn expand_variables(&self, mut src: &str, env: &mut dyn Env) -> Result<String> {
        let mut buf = String::new();

        while !src.is_empty() {
            let Some(index) = src.find('$') else {
                buf.push_str(src);
                break;
            };

            buf.push_str(&src[..index]);
            src = &src[index + 1..];

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
                    let index = find_ident_end(src);
                    if index == 0 {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: substitution syntax error",
                                &self.path, self.lineno
                            );
                        }
                        return Err(Error::syntax_error(self.lineno, 1));
                    }

                    let name = &src[..index].split('\0').next().unwrap();
                    src = &src[index..];
                    let value = env.get(name.as_ref());

                    if src.starts_with(":?") {
                        // required error when empty or unset
                        if let Some(value) = value {
                            if value.is_empty() {
                                if self.debug {
                                    eprintln!("{DEBUG_PREFIX}{}:{}: variable ${} may not be empty",
                                        &self.path, self.lineno, name
                                    );
                                }
                                return Err(Error::syntax_error(self.lineno, 1));
                            }
                            buf.push_str(value.to_string_lossy().as_ref());
                        } else {
                            if self.debug {
                                eprintln!("{DEBUG_PREFIX}{}:{}: variable ${} may not be unset",
                                    &self.path, self.lineno, name
                                );
                            }
                            return Err(Error::syntax_error(self.lineno, 1));
                        }
                        src = &src[2..];
                    } else if src.starts_with('?') {
                        // required error when unset
                        if let Some(value) = value {
                            buf.push_str(value.to_string_lossy().as_ref());
                        } else {
                            if self.debug {
                                eprintln!("{DEBUG_PREFIX}{}:{}: variable ${} may not be unset",
                                    &self.path, self.lineno, name
                                );
                            }
                            return Err(Error::syntax_error(self.lineno, 1));
                        }
                        src = &src[2..];
                    } else if src.starts_with(":-") {
                        // default when empty or unset
                        let index = src.find('}').unwrap_or(src.len());
                        let default = &src[2..index];
                        src = &src[index + 1..];
                        if let Some(value) = value {
                            if value.is_empty() {
                                buf.push_str(default);
                            } else {
                                buf.push_str(value.to_string_lossy().as_ref());
                            }
                        } else {
                            buf.push_str(default);
                        }
                    } else if src.starts_with('-') {
                        // default when unset
                        let index = src.find('}').unwrap_or(src.len());
                        let default = &src[2..index];
                        src = &src[index + 1..];
                        if let Some(value) = value {
                            buf.push_str(value.to_string_lossy().as_ref());
                        } else {
                            buf.push_str(default);
                        }
                    } else if src.starts_with(":+") {
                        // default when not empty
                        let index = src.find('}').unwrap_or(src.len());
                        let default = &src[2..index];
                        src = &src[index + 1..];
                        if let Some(value) = value {
                            if !value.is_empty() {
                                buf.push_str(default);
                            }
                        }
                    } else if src.starts_with('+') {
                        // default when set
                        let index = src.find('}').unwrap_or(src.len());
                        let default = &src[2..index];
                        src = &src[index + 1..];
                        if value.is_some() {
                            buf.push_str(default);
                        }
                    } else if src.starts_with('}') {
                        src = &src[1..];
                        if let Some(value) = value {
                            buf.push_str(value.to_string_lossy().as_ref());
                        }
                    } else {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: substitution syntax error",
                                &self.path, self.lineno
                            );
                        }
                        return Err(Error::syntax_error(self.lineno, 1));
                    }

                    if !src.starts_with('}') {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: expected }}",
                                &self.path, self.lineno
                            );
                        }
                        return Err(Error::syntax_error(self.lineno, 1));
                    }
                    src = &src[1..];
                }
                _ => {
                    let index = find_ident_end(src);
                    if index == 0 {
                        if self.debug {
                            eprintln!("{DEBUG_PREFIX}{}:{}: substitution syntax error",
                                &self.path, self.lineno
                            );
                        }
                        return Err(Error::syntax_error(self.lineno, 1));
                    }

                    let name = &src[..index].split('\0').next().unwrap();
                    src = &src[index..];

                    if let Some(value) = env.get(name.as_ref()) {
                        buf.push_str(value.to_string_lossy().as_ref());
                    }
                }
            }
        }

        Ok(buf)
    }
}

#[inline]
fn find_ident_end(src: &str) -> usize {
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

        src = &src[1..];

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
                let mut byte = 0u8;
                for _ in 0..3 {
                    let Some(val) = oct_head(src) else { break; };
                    byte *= 8;
                    byte += val;
                    src = &src[1..];
                }
                buf.push(byte as char);
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

fn oct_head(src: &str) -> Option<u8> {
    let ch = src.chars().next()?;

    if ch < '0' || ch > '7' {
        return None;
    }

    Some(ch as u8 - b'0')
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
