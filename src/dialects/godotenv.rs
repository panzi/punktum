use std::{borrow::Cow, io::BufRead, num::NonZeroU8, path::Path};

use crate::{env::GetEnv, Env, Error, Options, Result, DEBUG_PREFIX};

// trying to be compatible to: https://github.com/joho/godotenv/blob/v1.5.1/parser.go
pub fn config_godotenv(reader: &mut dyn BufRead, env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();

    let mut src = String::new();
    options.encoding.read_to_string(reader, &mut src)?;

    let src = src.replace("\r\n", "\n");
    let mut cutset = &src[..];
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

        let (key, left) = parser.locate_key_name(cutset)?;

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

        let (value, left) = parser.extract_var_value(left, env.as_get_env())?;
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

    fn locate_key_name<'b>(&self, mut src: &'b str) -> Result<(&'b str, &'b str)> {
        let mut key = "";
        src = self.trim_export(src);

        let mut offset = 0;
        for (index, rune) in src.char_indices() {
            if is_space(rune) {
                continue;
            }

            match rune {
                '='|':' => {
                    key = &src[..index];
                    offset = index + 1;
                    break;
                }
                '_' => {}
                _ => {
                    if rune.is_alphanumeric() || rune == '.' {
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

        Ok((key, cutset))
    }

    fn extract_var_value<'b>(&mut self, src: &'b str, env: &dyn GetEnv) -> Result<(String, &'b str)> {
        let Some(quote) = has_quote_prefix(src) else {
            if src.is_empty() {
                return Ok((String::new(), ""));
            }

            let end_of_line = src.find(|ch| ch == '\r' || ch == '\n').unwrap_or(src.len());
            let line = &src[..end_of_line];
            if line.is_empty() {
                return Ok((String::new(), &src[end_of_line..]));
            }

            // This is all weird and IMO wrong, but it is like it is in godotenv.
            // IMO this should NOT be scanned from the back. What if there is a # within a comment? Like:
            //
            //    FOO=BAR # commented out value # comment
            //
            // This will return the value: "BAR # commented out value"
            //
            // See: https://github.com/joho/godotenv/blob/3fc4292b58a67b78e1dbb6e47b4879a6cc602ec4/parser.go#L120
            let mut end_of_var = end_of_line;
            let mut iter = line.char_indices().rev().peekable();
            while let Some((index, ch)) = iter.next() {
                if ch == '#' && iter.peek().map(|(_, ch)| is_space(*ch)).unwrap_or(false) {
                    end_of_var = index;
                    break;
                }
            }

            let trimmed = src[..end_of_var].trim_end_matches(is_space);

            return Ok((trimmed.to_owned(), &src[end_of_line..]));
        };

        let quote = quote.get() as char;
        let src = &src[1..];
        for (index, ch) in src.char_indices() {
            if ch == '\n' {
                self.lineno += 1;
            }
            if ch != quote || (index > 0 && src[index - 1..].starts_with('\\')) {
                continue;
            }

            let value = src[..index].trim_matches(quote);

            if quote == '"' {
                let res = self.expand_variables(&expand_escapes(&value), env)?;
                return Ok((res, &src[index + ch.len_utf8()..]));
            }

            return Ok((value.to_owned(), &src[index + ch.len_utf8()..]));
        }

        let val_end_index = src.find('\n').unwrap_or(src.len());
        if self.debug {
            eprintln!("{DEBUG_PREFIX}{}:{}: unterminated quoted value {}",
                &self.path, self.lineno, &src[..val_end_index]
            );
        }

        Err(Error::syntax_error(self.lineno, 1))
    }

    fn expand_variables(&self, mut src: &str, env: &dyn GetEnv) -> Result<String> {
        let mut buf = String::new();

        while !src.is_empty() {
            let Some(index) = src.find('$') else {
                buf.push_str(src);
                break;
            };

            if index > 0 && src[index - 1..].starts_with('\\') {
                buf.push_str(&src[..index - 1]);
                buf.push('$');
                src = &src[index + 1..];
                continue;
            }

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

                    if !src.starts_with('}') {
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

                    let value = env.get(name.as_ref());
                    if let Some(value) = value {
                        buf.push_str(value.to_string_lossy().as_ref());
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

        src = &src[ch.len_utf8()..];

        match ch {
            'n' => buf.push('\n'),
            'r' => buf.push('\r'),
            '$' => buf.push_str("\\$"),
            _ => {
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
