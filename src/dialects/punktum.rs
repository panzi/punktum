use std::{borrow::Cow, fs::File, io::BufReader, path::Path};

use crate::{env::GetEnv, error::SourceLocation, Encoding, Env, Error, ErrorKind, Options, Result, DEBUG_PREFIX};

#[inline]
fn is_word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[inline]
fn skip_ws(src: &str, index: usize) -> usize {
    let len = src.len();
    if index >= len {
        return len;
    }
    src[index..].find(|ch: char| !ch.is_ascii_whitespace()).
        map(|pos| pos + index).
        unwrap_or(len)
}

#[inline]
fn find_word_end(src: &str, index: usize) -> usize {
    let len = src.len();
    if index >= len {
        return len;
    }
    src[index..].find(|ch: char| !is_word(ch)).
        map(|pos| pos + index).
        unwrap_or(len)
}

#[inline]
fn char_at(src: &str, index: usize) -> Option<char> {
    if index >= src.len() {
        return None;
    }
    src[index..].chars().next()
}

pub fn config_punktum(env: &mut dyn Env, parent: &dyn GetEnv, options: &Options<&Path>) -> Result<()> {
    let file = File::open(options.path);
    let path_str = options.path.to_string_lossy();

    match file {
        Err(err) => {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}: {err}");
            }
            if options.strict {
                return Err(Error::with_cause(ErrorKind::IOError, err));
            }
        }
        Ok(file) => {
            let mut key = String::new();
            let mut value = String::new();
            let mut parser = Parser {
                path: path_str,
                lineno: 0,
                debug: options.debug,
                strict: options.strict,
                encoding: options.encoding,
                linebuf: String::new(),
                reader: BufReader::new(file),
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
                parser.parse_value(index, &mut value, env)?;

                options.set_var(env, key.as_ref(), value.as_ref());
            }
        }
    }

    Ok(())
}

struct Parser<'c> {
    path: Cow<'c, str>,
    lineno: usize,
    debug: bool,
    strict: bool,
    encoding: Encoding,
    reader: BufReader<File>,
    linebuf: String,
}

impl<'c> Parser<'c> {
    fn parse_value(&mut self, mut index: usize, value: &mut String, env: &mut dyn Env) -> Result<usize> {
        while let Some(mut ch) = char_at(&self.linebuf, index) {
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
                            break;
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
                                        index += ch.len_utf8();
                                        if self.debug {
                                            let escseq = &self.linebuf[(index - 2)..index];
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
                            index = self.parse_var(&self.linebuf, index + 1, value, env)?;
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
            } else if ch == '#' {
                break;
            } else if ch == '$' {
                index = self.parse_var(&self.linebuf, index + 1, value, env)?;
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

                let mut eol = false;
                loop {
                    let next_index = skip_ws(&self.linebuf, index);
                    let Some(ch) = char_at(&self.linebuf, next_index) else {
                        // ignore trailing space
                        eol = true;
                        break;
                    };

                    if ch == '#' {
                        // ignore trailing space before comment
                        eol = true;
                        break;
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

                if eol {
                    break;
                }
            }
        }
        Ok(index)
    }

    fn parse_var(&'c self, src: &str, mut index: usize, buf: &mut String, env: &mut dyn Env) -> Result<usize> {
        let var_start_index = index - 1;
        let brace = src[index..].starts_with('{');
        if brace {
            index += 1;
        }
        let end_index = find_word_end(src, index);

        if brace && !src[end_index..].starts_with('}') {
            let column = var_start_index + 1;
            if self.debug {
                let line = src.trim_end_matches('\n');
                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected '}}': {line}", self.path, self.lineno);
            }
            if self.strict {
                return Err(Error::syntax_error(self.lineno, column));
            }
            index = end_index;
            buf.push_str(&src[var_start_index..index]);
        } else if end_index == index {
            let column = var_start_index + 1;
            if brace {
                if self.debug {
                    let line = src.trim_end_matches('\n');
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: ${{}} found: {line}", self.path, self.lineno);
                }

                index = end_index + 1;
            } else {
                if self.debug {
                    let line = src.trim_end_matches('\n');
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: single $ found: {line}", self.path, self.lineno);
                }

                index = end_index;
            }

            if self.strict {
                return Err(Error::syntax_error(self.lineno, column));
            }

            buf.push_str(&src[var_start_index..index]);
        } else {
            if let Some(val) = env.get(src[index..end_index].as_ref()) {
                buf.push_str(val.to_string_lossy().as_ref());
            }
            index = if brace {
                end_index + 1
            } else {
                end_index
            };
        }

        Ok(index)
    }
}
