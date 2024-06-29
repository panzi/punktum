use std::{borrow::Cow, io::BufRead, path::Path};

use crate::{env::{EmptyEnv, GetEnv}, error::SourceLocation, Encoding, Env, Error, ErrorKind, Options, Result, DEBUG_PREFIX};

#[inline]
fn is_word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[inline]
fn skip_ws(src: &str, index: usize) -> usize {
    let Some(slice) = src.get(index..) else {
        return src.len();
    };
    let Some(pos) = slice.find(|ch: char| !ch.is_ascii_whitespace()) else {
        return src.len();
    };
    pos + index
}

#[inline]
fn find_word_end(src: &str, index: usize) -> usize {
    let Some(slice) = src.get(index..) else {
        return src.len();
    };
    let Some(pos) = slice.find(|ch: char| !is_word(ch)) else {
        return src.len();
    };
    pos + index
}

#[inline]
fn char_at(src: &str, index: usize) -> Option<char> {
    src.get(index..)?.chars().next()
}

pub fn config_punktum(reader: &mut dyn BufRead, env: &mut dyn Env, parent: &dyn GetEnv, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();

    let mut key = String::new();
    let mut value = String::new();
    let mut parser = Parser {
        path: path_str,
        lineno: 0,
        debug: options.debug,
        strict: options.strict,
        encoding: options.encoding,
        linebuf: String::new(),
        reader,
    };

    loop {
        parser.linebuf.clear();
        parser.lineno += 1;
        if let Err(err) = options.encoding.read_line(&mut parser.reader, &mut parser.linebuf) {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{}:{}:1: {err}", parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(parser.lineno, 1)));
            }
            if err.kind() == std::io::ErrorKind::InvalidData {
                continue;
            } else {
                return Ok(());
            }
        }

        if parser.linebuf.is_empty() {
            break;
        }

        if parser.linebuf.ends_with("\r\n") {
            // convert DOS line endings to Unix
            parser.linebuf.remove(parser.linebuf.len() - 2);
        }

        let mut index = skip_ws(&parser.linebuf, 0);

        let Some(mut ch) = char_at(&parser.linebuf, index) else {
            continue;
        };

        if ch == '#' {
            continue;
        }

        let prev_index = index;

        if !is_word(ch) {
            let column = prev_index + 1;
            if options.debug {
                let line = parser.linebuf.trim_end_matches('\n');
                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected {ch:?}, expected variable name: {line}", parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, column));
            }
            continue;
        }

        index = find_word_end(&parser.linebuf, index);

        if index == prev_index {
            let column = prev_index + 1;
            if options.debug {
                let line = parser.linebuf.trim_end_matches('\n');
                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected end of line, expected variable name: {line}", parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, column));
            }
            continue;
        };

        key.clear();
        key.push_str(&parser.linebuf[prev_index..index]);

        if key.is_empty() {
            let column = index + 1;
            if options.debug {
                let line = parser.linebuf.trim_end_matches('\n');
                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected variable name: {line}", parser.path, parser.lineno);
            }
            if options.strict {
                return Err(Error::syntax_error(parser.lineno, column));
            }
            continue;
        }

        index = skip_ws(&parser.linebuf, index);

        {
            let Some(next_ch) = char_at(&parser.linebuf, index) else {
                if let Some(value) = parent.get(key.as_ref()) {
                    options.set_var(env, key.as_ref(), value.as_ref());
                }
                continue;
            };
            ch = next_ch;
        }

        if ch == '#' {
            if let Some(value) = parent.get(key.as_ref()) {
                options.set_var(env, key.as_ref(), value.as_ref());
            }
            continue;
        }

        if ch != '=' {
            if !options.strict && key.eq("export") && is_word(ch) {
                // allow `export FOO=BAR`
                key.clear();

                let prev_index = index;
                index = find_word_end(&parser.linebuf, index);

                key.push_str(&parser.linebuf[prev_index..index]);

                index = skip_ws(&parser.linebuf, index);
                {
                    let Some(next_ch) = char_at(&parser.linebuf, index) else {
                        if let Some(value) = parent.get(key.as_ref()) {
                            options.set_var(env, key.as_ref(), value.as_ref());
                        }
                        continue;
                    };
                    ch = next_ch
                }

                if ch == '#' {
                    if let Some(value) = parent.get(key.as_ref()) {
                        options.set_var(env, key.as_ref(), value.as_ref());
                    }
                    continue;
                }

                if ch != '=' {
                    let column = index + 1;
                    if options.debug {
                        let line = parser.linebuf.trim_end_matches('\n');
                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected '=', actual {ch:?}: {line}", parser.path, parser.lineno);
                    }
                    if options.strict {
                        return Err(Error::syntax_error(parser.lineno, column));
                    }
                    continue;
                }
            } else {
                let column = index + 1;
                if options.debug {
                    let line = parser.linebuf.trim_end_matches('\n');
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected '=', actual {ch:?}: {line}", parser.path, parser.lineno);
                }
                if options.strict {
                    return Err(Error::syntax_error(parser.lineno, column));
                }
                continue;
            }
        }

        index = skip_ws(&parser.linebuf, index + 1);

        value.clear();
        parser.parse_value(index, &mut value, env.as_get_env(), false)?;

        options.set_var(env, key.as_ref(), value.as_ref());
    }

    Ok(())
}

struct Parser<'c> {
    path: Cow<'c, str>,
    lineno: usize,
    debug: bool,
    strict: bool,
    encoding: Encoding,
    reader: &'c mut dyn BufRead,
    linebuf: String,
}

macro_rules! parse_var_error {
    ($self:expr, $index:expr, $buf:expr, $key:expr, $env:expr, $message:expr) => {
        if $buf.is_skipped() {
            $index = $self.parse_value($index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
        } else {
            let lineno = $self.lineno;
            let column = $index + 1;
            if $self.debug {
                // abusing the buffer so not to make yet another allocation
                let key_index = $buf.len();
                $buf.push_str($key);
                let message_index = $buf.len();
                $self.parse_value($index, $buf, $env, true)?;
                let message = $buf.tail(message_index);
                if message.is_empty() {
                    eprintln!("{DEBUG_PREFIX}{}:{}: variable ${} {}",
                        &$self.path, $self.lineno, $buf.slice(key_index, message_index),
                        $message
                    );
                } else {
                    eprintln!("{DEBUG_PREFIX}{}:{}: {}",
                        &$self.path, $self.lineno, message
                    );
                }
                $buf.truncate(key_index);
            }
            return Err(Error::substitution_error(lineno, column));
        }
    };
}

impl<'c> Parser<'c> {
    fn parse_value(&mut self, mut index: usize, value: &mut dyn StringBuffer, env: &dyn GetEnv, nested: bool) -> Result<usize> {
        loop {
            if nested && index >= self.linebuf.len() {
                index = 0;

                self.linebuf.clear();
                self.lineno += 1;
                if let Err(err) = self.encoding.read_line(self.reader, &mut self.linebuf) {
                    if self.debug {
                        eprintln!("{DEBUG_PREFIX}{}:{}:1: {err}", self.path, self.lineno);
                    }
                    if self.strict {
                        return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(self.lineno, 1)));
                    }
                    if err.kind() == std::io::ErrorKind::InvalidData {
                        break;
                    } else {
                        return Ok(index);
                    }
                }

                if self.linebuf.is_empty() {
                    return Ok(index);
                }
            }

            let Some(mut ch) = char_at(&self.linebuf, index) else {
                break;
            };

            #[allow(clippy::if_same_then_else)]
            if ch == '"' || ch == '\'' {
                let quote = ch;
                index += 1;
                let mut prev_index = index;

                loop {
                    {
                        let Some(next_ch) = char_at(&self.linebuf, index) else {
                            let column = prev_index + 1;
                            if self.debug {
                                let line = self.linebuf.trim_end_matches('\n');
                                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unterminated string literal: {line}", self.path, self.lineno);
                            }
                            if self.strict {
                                return Err(Error::syntax_error(self.lineno, column));
                            }
                            value.push_str(&self.linebuf[prev_index..]);
                            index = self.linebuf.len();
                            return Ok(index);
                        };
                        ch = next_ch;
                    }

                    if ch == quote {
                        if index > prev_index {
                            value.push_str(&self.linebuf[prev_index..index]);
                        }
                        index += 1;
                        break;
                    }

                    match ch {
                        '\\' if quote == '"' => {
                            if index > prev_index {
                                value.push_str(&self.linebuf[prev_index..index]);
                            }
                            index += 1;

                            if let Some(ch) = char_at(&self.linebuf, index) {
                                match ch {
                                    '\\' => {
                                        value.push('\\');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    '"' => {
                                        value.push('"');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    '$' => {
                                        value.push('$');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    '\'' => {
                                        value.push('\'');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    'r' => {
                                        value.push('\r');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    'n' => {
                                        value.push('\n');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    't' => {
                                        value.push('\t');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    'f' => {
                                        value.push('\x0C');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    'b' => {
                                        value.push('\x08');
                                        index += 1;
                                        prev_index = index;
                                    }
                                    'u' if self.linebuf.len() >= index + 5 => {
                                        index += 1;
                                        let unicode = &self.linebuf[index..index + 4];
                                        if let Ok(hi) = u16::from_str_radix(unicode, 16) {
                                            if hi >= 0xD800 && hi <= 0xDBFF {
                                                if self.linebuf.len() < index + 10 || !self.linebuf[index + 4..].starts_with("\\u") {
                                                    let column = index - 1;
                                                    if self.debug {
                                                        let escseq = &self.linebuf[(index - 2)..index + 4];
                                                        let line = self.linebuf.trim_end_matches('\n');
                                                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                                    }
                                                    if self.strict {
                                                        return Err(Error::syntax_error(self.lineno, column));
                                                    }
                                                } else {
                                                    let unicode = &self.linebuf[index + 6..index + 10];
                                                    if let Ok(lo) = u16::from_str_radix(unicode, 16) {
                                                        if lo < 0xDC00 || lo > 0xDFFF {
                                                            let column = index + 3;
                                                            if self.debug {
                                                                let escseq = &self.linebuf[(index + 2)..index + 10];
                                                                let line = self.linebuf.trim_end_matches('\n');
                                                                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                                            }
                                                            if self.strict {
                                                                return Err(Error::syntax_error(self.lineno, column));
                                                            }
                                                        } else {
                                                            let unicode = (((hi & 0x3ff) as u32) << 10 | (lo & 0x3ff) as u32) + 0x1_0000;
                                                            let unicode = unsafe { char::from_u32_unchecked(unicode) };
                                                            value.push(unicode);
                                                            index += 10;
                                                            prev_index = index;
                                                        }
                                                    } else {
                                                        let column = index + 1;
                                                        if self.debug {
                                                            let escseq = &self.linebuf[(index - 2)..index + 10];
                                                            let line = self.linebuf.trim_end_matches('\n');
                                                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                                        }
                                                        if self.strict {
                                                            return Err(Error::syntax_error(self.lineno, column));
                                                        }
                                                    }
                                                }
                                            } else if let Some(unicode) = char::from_u32(hi.into()) {
                                                value.push(unicode);
                                                index += 4;
                                                prev_index = index;
                                            } else {
                                                let column = index - 1;
                                                if self.debug {
                                                    let escseq = &self.linebuf[(index - 2)..index + 4];
                                                    let line = self.linebuf.trim_end_matches('\n');
                                                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                                }
                                                if self.strict {
                                                    return Err(Error::syntax_error(self.lineno, column));
                                                }
                                            }
                                        } else {
                                            let column = index - 1;
                                            if self.debug {
                                                let escseq = &self.linebuf[(index - 2)..index + 4];
                                                let line = self.linebuf.trim_end_matches('\n');
                                                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                            }
                                            if self.strict {
                                                return Err(Error::syntax_error(self.lineno, column));
                                            }
                                        }
                                    }
                                    'U' if self.linebuf.len() >= index + 7 => {
                                        index += 1;
                                        let unicode = &self.linebuf[index..index + 6];
                                        if let Ok(unicode) = u32::from_str_radix(unicode, 16) {
                                            if let Some(unicode) = char::from_u32(unicode) {
                                                value.push(unicode);
                                                index += 6;
                                                prev_index = index;
                                            } else {
                                                let column = index - 1;
                                                if self.debug {
                                                    let escseq = &self.linebuf[(index - 2)..index + 6];
                                                    let line = self.linebuf.trim_end_matches('\n');
                                                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                                }
                                                if self.strict {
                                                    return Err(Error::syntax_error(self.lineno, column));
                                                }
                                            }
                                        } else {
                                            let column = index - 1;
                                            if self.debug {
                                                let escseq = &self.linebuf[(index - 2)..index + 6];
                                                let line = self.linebuf.trim_end_matches('\n');
                                                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                            }
                                            if self.strict {
                                                return Err(Error::syntax_error(self.lineno, column));
                                            }
                                        }
                                    }
                                    '\0' => {
                                        index += 1;
                                        let column = index - 1;
                                        if self.debug {
                                            let line = self.linebuf.trim_end_matches('\n');
                                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal null byte: {line:?}", self.path, self.lineno);
                                        }
                                        if self.strict {
                                            return Err(Error::syntax_error(self.lineno, column));
                                        }
                                        value.push('\\');
                                        prev_index = index;
                                    }
                                    '\n' => {
                                        index = 0;
                                        prev_index = index;

                                        self.linebuf.clear();
                                        self.lineno += 1;
                                        if let Err(err) = self.encoding.read_line(&mut self.reader, &mut self.linebuf) {
                                            if self.debug {
                                                eprintln!("{DEBUG_PREFIX}{}:{}:1: {err}", self.path, self.lineno);
                                            }
                                            if self.strict {
                                                return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(self.lineno, 1)));
                                            }
                                            if err.kind() == std::io::ErrorKind::InvalidData {
                                                break;
                                            } else {
                                                return Ok(index);
                                            }
                                        }

                                        if self.linebuf.is_empty() {
                                            if self.debug {
                                                let line = self.linebuf.trim_end_matches('\n');
                                                eprintln!("{DEBUG_PREFIX}{}:{}:1: syntax error: unterminated string literal: {line}", self.path, self.lineno);
                                            }
                                            if self.strict {
                                                return Err(Error::syntax_error(self.lineno, 1));
                                            }
                                            return Ok(index);
                                        }
                                    }
                                    _ => {
                                        let column = index + 1;
                                        prev_index = index - 1;
                                        index += ch.len_utf8();
                                        if self.debug {
                                            let escseq = &self.linebuf[(index - 1 - ch.len_utf8())..index];
                                            let line = self.linebuf.trim_end_matches('\n');
                                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", self.path, self.lineno);
                                        }
                                        if self.strict {
                                            return Err(Error::syntax_error(self.lineno, column));
                                        }
                                    }
                                }
                            } else { // no '\n' means EOF
                                let column = index + 1;
                                if self.debug {
                                    let line = self.linebuf.trim_end_matches('\n');
                                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected end of file within escape seqeunce: {line}", self.path, self.lineno);
                                }
                                if self.strict {
                                    return Err(Error::syntax_error(self.lineno, column));
                                }
                                value.push('\\');
                                return Ok(index);
                            }
                        }
                        '$' if quote == '"' => {
                            value.push_str(&self.linebuf[prev_index..index]);
                            index = self.parse_var(index + 1, value, env)?;
                            prev_index = index;
                        }
                        '\n' => {
                            index += 1;
                            value.push_str(&self.linebuf[prev_index..index]);
                            index = 0;
                            prev_index = index;

                            self.linebuf.clear();
                            self.lineno += 1;
                            if let Err(err) = self.encoding.read_line(&mut self.reader, &mut self.linebuf) {
                                if self.debug {
                                    eprintln!("{DEBUG_PREFIX}{}:{}:1: {err}", self.path, self.lineno);
                                }
                                if self.strict {
                                    return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(self.lineno, 1)));
                                }
                                if err.kind() == std::io::ErrorKind::InvalidData {
                                    break;
                                } else {
                                    return Ok(index);
                                }
                            }

                            if self.linebuf.is_empty() {
                                if self.debug {
                                    eprintln!("{DEBUG_PREFIX}{}:{}:1: syntax error: unexpected end of file in string literal", self.path, self.lineno);
                                }
                                if self.strict {
                                    return Err(Error::syntax_error(self.lineno, 1));
                                }
                                return Ok(index);
                            }
                        }
                        '\0' => {
                            let column = index + 1;
                            if self.debug {
                                let line = self.linebuf.trim_end_matches('\n');
                                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal null byte: {line:?}", self.path, self.lineno);
                            }
                            if self.strict {
                                return Err(Error::syntax_error(self.lineno, column));
                            }
                            if index > prev_index {
                                value.push_str(&self.linebuf[prev_index..index]);
                            }
                            index += 1;
                            prev_index = index;
                        }
                        _ => {
                            index += ch.len_utf8();
                        }
                    }
                }
            } else if ch == '#' && !nested {
                break;
            } else if ch == '}' && nested {
                break;
            } else if ch == '$' {
                index = self.parse_var(index + 1, value, env)?;
            } else if ch == '\0' {
                let column = index + 1;
                if self.debug {
                    let line = self.linebuf.trim_end_matches('\n');
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal null byte: {line:?}", self.path, self.lineno);
                }
                if self.strict {
                    return Err(Error::syntax_error(self.lineno, column));
                }
                index += 1;
            } else if nested {
                let mut prev_index = index;

                loop {
                    let next_index = skip_ws(&self.linebuf, index);
                    let Some(ch) = char_at(&self.linebuf, next_index) else {
                        value.push_str(&self.linebuf[prev_index..]);

                        index = 0;
                        prev_index = index;
                        self.linebuf.clear();
                        self.lineno += 1;

                        if let Err(err) = self.encoding.read_line(&mut self.reader, &mut self.linebuf) {
                            if self.debug {
                                eprintln!("{DEBUG_PREFIX}{}:{}:1: {err}", self.path, self.lineno);
                            }
                            if self.strict {
                                return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(self.lineno, 1)));
                            }
                            if err.kind() == std::io::ErrorKind::InvalidData {
                                continue;
                            } else {
                                return Ok(index);
                            }
                        }

                        if self.linebuf.is_empty() {
                            return Ok(index);
                        }

                        continue;
                    };

                    index = next_index;
                    if ch == '"' || ch == '\'' || ch == '\0' || ch == '$' || ch == '}' {
                        break;
                    }
                    index += ch.len_utf8();
                }

                if index > prev_index {
                    value.push_str(&self.linebuf[prev_index..index]);
                }
            } else {
                let prev_index = index;

                if ch.is_ascii_whitespace() {
                    index = skip_ws(&self.linebuf, index + 1);

                    let Some(ch) = char_at(&self.linebuf, index) else {
                        // ignore trailing space
                        break;
                    };

                    if ch == '#' {
                        // ignore trailing space before comment
                        break;
                    }

                    if ch == '"' || ch == '\'' || ch == '\0' || ch == '$' {
                        if index > prev_index {
                            value.push_str(&self.linebuf[prev_index..index]);
                        }
                        continue;
                    }
                }

                loop {
                    let next_index = skip_ws(&self.linebuf, index);
                    let Some(ch) = char_at(&self.linebuf, next_index) else {
                        // ignore trailing space
                        if index > prev_index {
                            value.push_str(&self.linebuf[prev_index..index]);
                        }
                        index = self.linebuf.len();
                        return Ok(index);
                    };

                    if ch == '#' {
                        // ignore trailing space before comment
                        if index > prev_index {
                            value.push_str(&self.linebuf[prev_index..index]);
                        }
                        return Ok(self.linebuf.len());
                    }

                    index = next_index;
                    if ch == '"' || ch == '\'' || ch == '\0' || ch == '$' {
                        break;
                    }
                    index += ch.len_utf8();
                }

                if index > prev_index {
                    value.push_str(&self.linebuf[prev_index..index]);
                }
            }
        }
        Ok(index)
    }

    fn parse_var(&mut self, mut index: usize, buf: &mut dyn StringBuffer, env: &dyn GetEnv) -> Result<usize> {
        let var_start_index = index - 1;
        let brace = self.linebuf[index..].starts_with('{');
        if brace {
            index += 1;
        }
        let end_index = find_word_end(&self.linebuf, index);

        if end_index == index {
            let column = var_start_index + 1;
            index = end_index;

            if self.debug {
                let line = self.linebuf.trim_end_matches('\n');
                if brace {
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: ${{}} with empty variable name: {line}", self.path, self.lineno);
                } else {
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: single $ found: {line}", self.path, self.lineno);
                }
            }

            if self.strict {
                return Err(Error::syntax_error(self.lineno, column));
            }

            if brace {
                index += 1;
            }

            buf.push_str(&self.linebuf[var_start_index..index]);

            return Ok(index);
        }

        let key = &self.linebuf[index..end_index];
        let value = env.get(key.as_ref());
        index = end_index;
        if brace {
            let tail = &self.linebuf[index..];

            if tail.starts_with(":?") {
                // required error when empty or unset
                index += 2;
                if let Some(value) = value {
                    if value.is_empty() {
                        parse_var_error!(self, index, buf, key, env, "may not be empty");
                    } else {
                        buf.push_str(value.to_string_lossy().as_ref());
                        index = self.parse_value(index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
                    }
                } else {
                    parse_var_error!(self, index, buf, key, env, "may not be unset");
                }
            } else if tail.starts_with('?') {
                // required error when unset
                index += 1;
                if let Some(value) = value {
                    buf.push_str(value.to_string_lossy().as_ref());
                    index = self.parse_value(index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
                } else {
                    parse_var_error!(self, index, buf, key, env, "may not be unset");
                }
            } else if tail.starts_with(":-") {
                // default when empty or unset
                index += 2;
                if let Some(value) = value {
                    if value.is_empty() {
                        index = self.parse_value(index, buf, env, true)?;
                    } else {
                        buf.push_str(value.to_string_lossy().as_ref());
                        index = self.parse_value(index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
                    }
                } else {
                    index = self.parse_value(index, buf, env, true)?;
                }
            } else if tail.starts_with('-') {
                // default when unset
                index += 1;
                if let Some(value) = value {
                    buf.push_str(value.to_string_lossy().as_ref());
                    index = self.parse_value(index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
                } else {
                    index = self.parse_value(index, buf, env, true)?;
                }
            } else if tail.starts_with(":+") {
                // default when not empty
                index += 2;
                if let Some(value) = value {
                    if value.is_empty() {
                        index = self.parse_value(index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
                    } else {
                        index = self.parse_value(index, buf, env, true)?;
                    }
                } else {
                    index = self.parse_value(index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
                }
            } else if tail.starts_with('+') {
                // default when set
                index += 1;
                if value.is_some() {
                    index = self.parse_value(index, buf, env, true)?;
                } else {
                    index = self.parse_value(index, &mut NullStringBuffer(), &EmptyEnv(), true)?;
                }
            } else if let Some(value) = value {
                buf.push_str(value.to_string_lossy().as_ref());
            }

            let tail = &self.linebuf[index..];
            if tail.starts_with('}') {
                index += 1;
            } else {
                let column = end_index + 1;
                if self.debug {
                    let line = self.linebuf.trim_end_matches('\n');
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected '}}': {line}", self.path, self.lineno);
                }
                if self.strict {
                    return Err(Error::syntax_error(self.lineno, column));
                }
                index = end_index;
                buf.push_str(&self.linebuf[var_start_index..index]);
            }
        } else if let Some(value) = value {
            // TODO: don't use lossy when strict?
            buf.push_str(value.to_string_lossy().as_ref());
        }

        Ok(index)
    }
}

trait StringBuffer {
    fn push(&mut self, ch: char);
    fn push_str(&mut self, string: &str);
    fn len(&self) -> usize;
    fn truncate(&mut self, new_len: usize);
    fn is_skipped(&self) -> bool;
    fn tail(&self, index: usize) -> &str;
    fn slice(&self, start_index: usize, end_index: usize) -> &str;
}

impl StringBuffer for String {
    #[inline]
    fn push(&mut self, ch: char) {
        String::push(self, ch);
    }

    #[inline]
    fn push_str(&mut self, string: &str) {
        String::push_str(self, string);
    }

    #[inline]
    fn len(&self) -> usize {
        String::len(self)
    }

    #[inline]
    fn truncate(&mut self, new_len: usize) {
        String::truncate(self, new_len);
    }

    #[inline]
    fn is_skipped(&self) -> bool {
        false
    }

    #[inline]
    fn tail(&self, index: usize) -> &str {
        &self[index..]
    }

    #[inline]
    fn slice(&self, start_index: usize, end_index: usize) -> &str {
        &self[start_index..end_index]
    }
}

struct NullStringBuffer();

impl StringBuffer for NullStringBuffer {
    #[inline]
    fn push(&mut self, _ch: char) {}

    #[inline]
    fn push_str(&mut self, _string: &str) {}

    #[inline]
    fn len(&self) -> usize { 0 }

    #[inline]
    fn truncate(&mut self, _new_len: usize) {}

    #[inline]
    fn is_skipped(&self) -> bool {
        true
    }

    #[inline]
    fn tail(&self, _index: usize) -> &str {
        ""
    }

    #[inline]
    fn slice(&self, _start_index: usize, _end_index: usize) -> &str {
        ""
    }
}
