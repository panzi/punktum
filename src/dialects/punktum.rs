use std::{borrow::Cow, fs::File, io::BufReader, path::Path};

use crate::{error::SourceLocation, Env, Error, ErrorKind, Options, Result, DEBUG_PREFIX};

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

pub fn config_punktum(env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
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
            let mut reader = BufReader::new(file);
            let mut buf = String::new();
            let mut key = String::new();
            let mut value = String::new();
            let mut parser = Parser {
                path: path_str,
                lineno: 0,
                debug: options.debug,
                strict: options.strict,
            };

            loop {
                buf.clear();
                parser.lineno += 1;
                if let Err(err) = options.encoding.read_line(&mut reader, &mut buf) {
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

                if buf.is_empty() {
                    break;
                }

                if buf.ends_with("\r\n") {
                    // convert DOS line endings to Unix
                    buf.remove(buf.len() - 2);
                }

                let mut index = skip_ws(&buf, 0);

                let Some(mut ch) = char_at(&buf, index) else {
                    continue;
                };

                if ch == '#' {
                    continue;
                }

                let prev_index = index;

                if !is_word(ch) {
                    let column = prev_index + 1;
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected {ch:?}, expected variable name: {line}", parser.path, parser.lineno);
                    }
                    if options.strict {
                        return Err(Error::syntax_error(parser.lineno, column));
                    }
                    continue;
                }

                index = find_word_end(&buf, index);

                if index == prev_index {
                    let column = prev_index + 1;
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected end of line, expected variable name: {line}", parser.path, parser.lineno);
                    }
                    if options.strict {
                        return Err(Error::syntax_error(parser.lineno, column));
                    }
                    continue;
                };

                key.clear();
                key.push_str(&buf[prev_index..index]);

                if key.is_empty() {
                    let column = index + 1;
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected variable name: {line}", parser.path, parser.lineno);
                    }
                    if options.strict {
                        return Err(Error::syntax_error(parser.lineno, column));
                    }
                    continue;
                }

                index = skip_ws(&buf, index);

                {
                    let Some(next_ch) = char_at(&buf, index) else {
                        let column = index + 1;
                        if options.debug {
                            let line = buf.trim_end_matches('\n');
                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected end of line, expected '=': {line}", parser.path, parser.lineno);
                        }
                        if options.strict {
                            return Err(Error::syntax_error(parser.lineno, column));
                        }
                        continue;
                    };
                    ch = next_ch;
                }

                if ch != '=' {
                    if !options.strict && key.eq("export") && is_word(ch) {
                        // allow `export FOO=BAR`
                        key.clear();

                        let prev_index = index;
                        index = find_word_end(&buf, index);

                        key.push_str(&buf[prev_index..index]);

                        if key.is_empty() {
                            let column = index + 1;
                            if options.debug {
                                let line = buf.trim_end_matches('\n');
                                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected variable name: {line}", parser.path, parser.lineno);
                            }
                            if options.strict {
                                return Err(Error::syntax_error(parser.lineno, column));
                            }
                            continue;
                        }

                        index = skip_ws(&buf, index);
                        {
                            let Some(next_ch) = char_at(&buf, index) else {
                                let column = index + 1;
                                if options.debug {
                                    let line = buf.trim_end_matches('\n');
                                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected end of line, expected '=': {line}", parser.path, parser.lineno);
                                }
                                if options.strict {
                                    return Err(Error::syntax_error(parser.lineno, column));
                                }
                                continue;
                            };
                            ch = next_ch
                        }

                        if ch != '=' {
                            let column = index + 1;
                            if options.debug {
                                let line = buf.trim_end_matches('\n');
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
                            let line = buf.trim_end_matches('\n');
                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected '=', actual {ch:?}: {line}", parser.path, parser.lineno);
                        }
                        if options.strict {
                            return Err(Error::syntax_error(parser.lineno, column));
                        }
                        continue;
                    }
                }

                index = skip_ws(&buf, index + 1);

                value.clear();
                while let Some(mut ch) = char_at(&buf, index) {
                    if ch == '"' || ch == '\'' {
                        let quote = ch;
                        index += 1;
                        let mut prev_index = index;

                        loop {
                            {
                                let Some(next_ch) = char_at(&buf, index) else {
                                    let column = prev_index + 1;
                                    if options.debug {
                                        let line = buf.trim_end_matches('\n');
                                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unterminated string literal: {line}", parser.path, parser.lineno);
                                    }
                                    if options.strict {
                                        return Err(Error::syntax_error(parser.lineno, column));
                                    }
                                    value.push_str(&buf[prev_index..]);
                                    index = buf.len();
                                    break;
                                };
                                ch = next_ch;
                            }

                            if ch == quote {
                                if index > prev_index {
                                    value.push_str(&buf[prev_index..index]);
                                }
                                index += 1;
                                break;
                            }

                            match ch {
                                '\\' if quote == '"' => {
                                    if index > prev_index {
                                        value.push_str(&buf[prev_index..index]);
                                    }
                                    index += 1;

                                    if let Some(ch) = char_at(&buf, index) {
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
                                            'u' if buf.len() >= index + 5 => {
                                                index += 1;
                                                let unicode = &buf[index..index + 4];
                                                if let Ok(hi) = u32::from_str_radix(unicode, 16) {
                                                    if hi >= 0xD800 && hi <= 0xDBFF {
                                                        if buf.len() < index + 10 || !buf[index + 4..].starts_with("\\u") {
                                                            let column = index - 1;
                                                            if options.debug {
                                                                let escseq = &buf[(index - 2)..index + 4];
                                                                let line = buf.trim_end_matches('\n');
                                                                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                            }
                                                            if options.strict {
                                                                return Err(Error::syntax_error(parser.lineno, column));
                                                            }
                                                        } else {
                                                            let unicode = &buf[index + 6..index + 10];
                                                            if let Ok(lo) = u32::from_str_radix(unicode, 16) {
                                                                if lo < 0xDC00 || lo > 0xDFFF {
                                                                    let column = index + 3;
                                                                    if options.debug {
                                                                        let escseq = &buf[(index + 2)..index + 10];
                                                                        let line = buf.trim_end_matches('\n');
                                                                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                                    }
                                                                    if options.strict {
                                                                        return Err(Error::syntax_error(parser.lineno, column));
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
                                                                if options.debug {
                                                                    let escseq = &buf[(index - 2)..index + 10];
                                                                    let line = buf.trim_end_matches('\n');
                                                                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                                }
                                                                if options.strict {
                                                                    return Err(Error::syntax_error(parser.lineno, column));
                                                                }
                                                            }
                                                        }
                                                    } else if let Some(unicode) = char::from_u32(hi) {
                                                        value.push(unicode);
                                                        index += 4;
                                                        prev_index = index;
                                                    } else {
                                                        let column = index - 1;
                                                        if options.debug {
                                                            let escseq = &buf[(index - 2)..index + 4];
                                                            let line = buf.trim_end_matches('\n');
                                                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                        }
                                                        if options.strict {
                                                            return Err(Error::syntax_error(parser.lineno, column));
                                                        }
                                                    }
                                                } else {
                                                    let column = index - 1;
                                                    if options.debug {
                                                        let escseq = &buf[(index - 2)..index + 4];
                                                        let line = buf.trim_end_matches('\n');
                                                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                    }
                                                    if options.strict {
                                                        return Err(Error::syntax_error(parser.lineno, column));
                                                    }
                                                }
                                            }
                                            'U' if buf.len() >= index + 7 => {
                                                index = index + 1;
                                                let unicode = &buf[index..index + 6];
                                                if let Ok(unicode) = u32::from_str_radix(unicode, 16) {
                                                    if let Some(unicode) = char::from_u32(unicode) {
                                                        value.push(unicode);
                                                        index += 6;
                                                        prev_index = index;
                                                    } else {
                                                        let column = index - 1;
                                                        if options.debug {
                                                            let escseq = &buf[(index - 2)..index + 6];
                                                            let line = buf.trim_end_matches('\n');
                                                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                        }
                                                        if options.strict {
                                                            return Err(Error::syntax_error(parser.lineno, column));
                                                        }
                                                    }
                                                } else {
                                                    let column = index - 1;
                                                    if options.debug {
                                                        let escseq = &buf[(index - 2)..index + 6];
                                                        let line = buf.trim_end_matches('\n');
                                                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                    }
                                                    if options.strict {
                                                        return Err(Error::syntax_error(parser.lineno, column));
                                                    }
                                                }
                                            }
                                            '\0' => {
                                                index += 1;
                                                let column = index - 1;
                                                if options.debug {
                                                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal null byte: {buf:?}", parser.path, parser.lineno);
                                                }
                                                if options.strict {
                                                    return Err(Error::syntax_error(parser.lineno, column));
                                                }
                                                value.push('\\');
                                                prev_index = index;
                                            }
                                            '\n' => {
                                                index = 0;
                                                prev_index = index;

                                                buf.clear();
                                                parser.lineno += 1;
                                                if let Err(err) = options.encoding.read_line(&mut reader, &mut buf) {
                                                    if options.debug {
                                                        eprintln!("{DEBUG_PREFIX}{}:{}:1: {err}", parser.path, parser.lineno);
                                                    }
                                                    if options.strict {
                                                        return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(parser.lineno, 1)));
                                                    }
                                                    if err.kind() == std::io::ErrorKind::InvalidData {
                                                        break;
                                                    } else {
                                                        options.set_var(env, key.as_ref(), value.as_ref());
                                                        return Ok(());
                                                    }
                                                }

                                                if buf.is_empty() {
                                                    if options.debug {
                                                        let line = buf.trim_end_matches('\n');
                                                        eprintln!("{DEBUG_PREFIX}{}:{}:1: syntax error: unterminated string literal: {line}", parser.path, parser.lineno);
                                                    }
                                                    if options.strict {
                                                        return Err(Error::syntax_error(parser.lineno, 1).into());
                                                    }
                                                    options.set_var(env, key.as_ref(), value.as_ref());
                                                    return Ok(());
                                                }
                                            }
                                            _ => {
                                                let column = index + 1;
                                                index += ch.len_utf8();
                                                if options.debug {
                                                    let escseq = &buf[(index - 2)..index];
                                                    let line = buf.trim_end_matches('\n');
                                                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}", parser.path, parser.lineno);
                                                }
                                                if options.strict {
                                                    return Err(Error::syntax_error(parser.lineno, column));
                                                }
                                            }
                                        }
                                    } else { // no '\n' means EOF
                                        let column = index + 1;
                                        if options.debug {
                                            let line = buf.trim_end_matches('\n');
                                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: unexpected end of file within escape seqeunce: {line}", parser.path, parser.lineno);
                                        }
                                        if options.strict {
                                            return Err(Error::syntax_error(parser.lineno, column));
                                        }
                                        value.push('\\');
                                        options.set_var(env, key.as_ref(), value.as_ref());
                                        return Ok(());
                                    }
                                }
                                '$' => {
                                    index = parser.parse_var(&buf, index + 1, &mut value, env)?;
                                }
                                '\n' => {
                                    index += 1;
                                    value.push_str(&buf[prev_index..index]);
                                    index = 0;
                                    prev_index = index;

                                    buf.clear();
                                    parser.lineno += 1;
                                    if let Err(err) = options.encoding.read_line(&mut reader, &mut buf) {
                                        if options.debug {
                                            eprintln!("{DEBUG_PREFIX}{}:{}:1: {err}", parser.path, parser.lineno);
                                        }
                                        if options.strict {
                                            return Err(Error::with_cause(ErrorKind::IOError, err));
                                        }
                                        if err.kind() == std::io::ErrorKind::InvalidData {
                                            break;
                                        } else {
                                            options.set_var(env, key.as_ref(), value.as_ref());
                                            return Ok(());
                                        }
                                    }

                                    if buf.is_empty() {
                                        if options.debug {
                                            eprintln!("{DEBUG_PREFIX}{}:{}:1: syntax error: unexpected end of file in string literal", parser.path, parser.lineno);
                                        }
                                        if options.strict {
                                            return Err(Error::syntax_error(parser.lineno, 1).into());
                                        }
                                        options.set_var(env, key.as_ref(), value.as_ref());
                                        return Ok(());
                                    }
                                }
                                '\0' => {
                                    let column = index + 1;
                                    if options.debug {
                                        let line = buf.trim_end_matches('\n');
                                        eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal null byte: {line:?}", parser.path, parser.lineno);
                                    }
                                    if options.strict {
                                        return Err(Error::syntax_error(parser.lineno, column));
                                    }
                                    if index > prev_index {
                                        value.push_str(&buf[prev_index..index]);
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
                        index = parser.parse_var(&buf, index + 1, &mut value, env)?;
                    } else if ch == '\0' {
                        let column = index + 1;
                        if options.debug {
                            let line = buf.trim_end_matches('\n');
                            eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: illegal null byte: {line:?}", parser.path, parser.lineno);
                        }
                        if options.strict {
                            return Err(Error::syntax_error(parser.lineno, column));
                        }
                        index += 1;
                    } else {
                        let prev_index = index;

                        if ch.is_ascii_whitespace() {
                            index = skip_ws(&buf, index + 1);

                            let Some(ch) = char_at(&buf, index) else {
                                // ignore trailing space
                                break;
                            };

                            if ch == '#' {
                                // ignore trailing space before comment
                                break;
                            }

                            if ch == '"' || ch == '\'' || ch == '\0' || ch == '$' {
                                if index > prev_index {
                                    value.push_str(&buf[prev_index..index]);
                                }
                                continue;
                            }
                        }

                        let mut eol = false;
                        loop {
                            let next_index = skip_ws(&buf, index);
                            let Some(ch) = char_at(&buf, next_index) else {
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
                            value.push_str(&buf[prev_index..index]);
                        }

                        if eol {
                            break;
                        }
                    }
                }

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
}

impl<'c> Parser<'c> {
    fn parse_var(&'c self, src: &str, mut index: usize, buf: &mut String, env: &mut dyn Env) -> Result<usize> {
        let var_start_index = index - 1;
        let brace = src[index..].starts_with('{');
        if brace {
            index += 1;
        }
        let end_index = find_word_end(&buf, index);

        if brace && !src[end_index..].starts_with('}') {
            let column = var_start_index + 1;
            if self.debug {
                let line = buf.trim_end_matches('\n');
                eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: expected '}}': {line}", self.path, self.lineno);
            }
            if self.strict {
                return Err(Error::syntax_error(self.lineno, column).into());
            }
            buf.push_str(&src[var_start_index..end_index]);
            index = end_index;
        } else if end_index == index {
            let column = var_start_index + 1;
            if brace {
                if self.debug {
                    let line = buf.trim_end_matches('\n');
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: ${{}} found: {line}", self.path, self.lineno);
                }

                index = end_index + 1;
            } else {
                if self.debug {
                    let line = buf.trim_end_matches('\n');
                    eprintln!("{DEBUG_PREFIX}{}:{}:{column}: syntax error: single $ found: {line}", self.path, self.lineno);
                }

                index = end_index;
            }

            if self.strict {
                return Err(Error::syntax_error(self.lineno, column).into());
            }
        } else {
            if let Some(val) = env.get(buf[index..end_index].as_ref()) {
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
