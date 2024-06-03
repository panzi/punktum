use std::str::CharIndices;
use std::{fs::File, io::BufReader, path::Path};

pub mod error;
pub use error::Error;
pub use error::ErrorKind;
use error::SourceLocation;

pub mod options;
pub use options::Options;
use options::Builder;

pub mod result;
pub use result::Result;

pub mod env;
pub use env::Env;
use env::{GetEnv, SystemEnv};

pub(crate) const DEBUG_PREFIX: &str = concat!("[", env!("CARGO_PKG_NAME"), "@", env!("CARGO_PKG_VERSION"), "][DEBUG] ");

#[inline]
pub fn build() -> Builder {
    Builder::new()
}

#[inline]
pub fn build_from<E: GetEnv>(env: &E) -> Result<Builder> {
    Builder::try_from(env)
}

#[inline]
pub fn build_from_env() -> Result<Builder> {
    Builder::try_from_env()
}

#[inline]
pub fn system_env() -> SystemEnv {
    SystemEnv::get()
}

#[inline]
pub fn config() -> Result<()> {
    let options = Options::try_from_env()?;
    config_with(&mut SystemEnv::get(), &options)
}

fn skipws(iter: &mut CharIndices) -> Option<(usize, char)> {
    while let Some((index, ch)) = iter.next() {
        if !ch.is_ascii_whitespace() {
            return Some((index, ch));
        }
    }

    None
}

fn skip_word(iter: &mut CharIndices) -> Option<(usize, char)> {
    while let Some((index, ch)) = iter.next() {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return Some((index, ch));
        }
    }

    None
}

#[inline]
pub fn config_with<P, E: Env>(env: &mut E, options: &Options<P>) -> Result<()>
where P: AsRef<Path> + Clone {
    config_with_intern(env, &Options {
        override_env: options.override_env,
        strict: options.strict,
        debug: options.debug,
        encoding: options.encoding,
        path: options.path.as_ref(),
    })
}

pub fn config_with_intern(env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
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
            let mut lineno: usize = 0;
            let mut buf = String::new();
            let mut key = String::new();
            let mut value = String::new();

            loop {
                buf.clear();
                lineno += 1;
                if let Err(err) = options.read_line(&mut reader, &mut buf) {
                    if options.debug {
                        eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:1: {err}");
                    }
                    if options.strict {
                        return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(lineno, 1)));
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

                let mut iter = buf.char_indices();

                let Some((index, ch)) = skipws(&mut iter) else {
                    continue;
                };

                if ch == '#' {
                    continue;
                }

                let prev_index = index;

                if !ch.is_ascii_alphanumeric() && ch != '_' {
                    let column = prev_index + 1;
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected {ch:?}, expected variable name: {line}");
                    }
                    if options.strict {
                        return Err(Error::syntax_error(lineno, column));
                    }
                    continue;
                }

                let Some((mut index, mut ch)) = skip_word(&mut iter) else {
                    let column = prev_index + 1;
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected end of line, expected variable name: {line}");
                    }
                    if options.strict {
                        return Err(Error::syntax_error(lineno, column));
                    }
                    continue;
                };

                key.clear();
                key.push_str(&buf[prev_index..index]);

                if key.is_empty() {
                    let column = index + 1;
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected {ch:?}: {line}");
                    }
                    if options.strict {
                        return Err(Error::syntax_error(lineno, column));
                    }
                    continue;
                }

                if ch != '=' {
                    if !ch.is_ascii_whitespace() {
                        let column = index + 1;
                        if options.debug {
                            let line = buf.trim_end_matches('\n');
                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected {ch:?}: {line}");
                        }
                        if options.strict {
                            return Err(Error::syntax_error(lineno, column));
                        }
                        continue;
                    }

                    let Some((next_index, next_ch)) = skipws(&mut iter) else {
                        let column = index + 1;
                        if options.debug {
                            let line = buf.trim_end_matches('\n');
                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected end of line, expected '=': {line}");
                        }
                        if options.strict {
                            return Err(Error::syntax_error(lineno, column));
                        }
                        continue;
                    };
                    index = next_index;
                    ch = next_ch;
                }

                if ch != '=' {
                    if !options.strict && key.eq("export") && (ch.is_ascii_alphanumeric() || ch == '_') {
                        // allow `export FOO=BAR`
                        key.clear();

                        let prev_index = index;
                        let Some((next_index, next_ch)) = skip_word(&mut iter) else {
                            let column = prev_index + 1;
                            if options.debug {
                                let line = buf.trim_end_matches('\n');
                                eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected end of line, expected variable name: {line}");
                            }
                            if options.strict {
                                return Err(Error::syntax_error(lineno, column));
                            }
                            continue;
                        };

                        index = next_index;
                        ch = next_ch;

                        key.push_str(&buf[prev_index..index]);

                        if key.is_empty() {
                            let column = index + 1;
                            if options.debug {
                                let line = buf.trim_end_matches('\n');
                                eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected {ch:?}: {line}");
                            }
                            if options.strict {
                                return Err(Error::syntax_error(lineno, column));
                            }
                            continue;
                        }

                        if ch.is_whitespace() {
                            let Some((next_index, next_ch)) = skipws(&mut iter) else {
                                let column = index + 1;
                                if options.debug {
                                    let line = buf.trim_end_matches('\n');
                                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected end of line, expected '=': {line}");
                                }
                                if options.strict {
                                    return Err(Error::syntax_error(lineno, column));
                                }
                                continue;
                            };
                            index = next_index;
                            ch = next_ch;
                        }

                        if ch != '=' {
                            let column = index + 1;
                            if options.debug {
                                let line = buf.trim_end_matches('\n');
                                eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: expected '=', actual {ch:?}: {line}");
                            }
                            if options.strict {
                                return Err(Error::syntax_error(lineno, column));
                            }
                            continue;
                        }
                    } else {
                        let column = index + 1;
                        if options.debug {
                            let line = buf.trim_end_matches('\n');
                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: expected '=', actual {ch:?}: {line}");
                        }
                        if options.strict {
                            return Err(Error::syntax_error(lineno, column));
                        }
                        continue;
                    }
                }

                value.clear();
                let Some((next_index, next_ch)) = skipws(&mut iter) else {
                    options.set_var(env, key.as_ref(), value.as_ref());
                    continue;
                };
                index = next_index;
                ch = next_ch;

                while index < buf.len() {
                    if ch == '"' || ch == '\'' {
                        let quote = ch;
                        let mut prev_index = index + 1;

                        loop {
                            let Some((next_index, next_ch)) = iter.next() else {
                                let column = prev_index + 1;
                                if options.debug {
                                    let line = buf.trim_end_matches('\n');
                                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unterminated string literal: {line}");
                                }
                                if options.strict {
                                    return Err(Error::syntax_error(lineno, column));
                                }
                                value.push_str(&buf[prev_index..]);
                                index = buf.len();
                                break;
                            };

                            if next_ch == quote {
                                if next_index > prev_index {
                                    value.push_str(&buf[prev_index..next_index]);
                                }
                                let Some((next_index, next_ch)) = iter.next() else {
                                    index = buf.len();
                                    break;
                                };
                                index = next_index;
                                ch = next_ch;
                                break;
                            }

                            match next_ch {
                                '\\' if quote == '"' => {
                                    if next_index > prev_index {
                                        value.push_str(&buf[prev_index..next_index]);
                                    }

                                    if let Some((next_index, next_ch)) = iter.next() {
                                        match next_ch {
                                            '\\' => {
                                                value.push('\\');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            '"' => {
                                                value.push('"');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            '\'' => {
                                                value.push('\'');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            'r' => {
                                                value.push('\r');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            'n' => {
                                                value.push('\n');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            't' => {
                                                value.push('\t');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            'f' => {
                                                value.push('\x0C');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            'b' => {
                                                value.push('\x08');
                                                index = next_index + 1;
                                                prev_index = index;
                                            }
                                            '\0' => {
                                                index = next_index + 1;
                                                let column = next_index + 1;
                                                if options.debug {
                                                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                                                }
                                                if options.strict {
                                                    return Err(Error::syntax_error(lineno, column));
                                                }
                                                value.push('\\');
                                                prev_index = index;
                                            }
                                            _ => {
                                                index = next_index + 1;
                                                let column = next_index + 1;
                                                if options.debug {
                                                    let escseq = &buf[(index - 2)..index];
                                                    let line = buf.trim_end_matches('\n');
                                                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}");
                                                }
                                                if options.strict {
                                                    return Err(Error::syntax_error(lineno, column));
                                                }
                                                if next_ch == '\n' {
                                                    value.push_str(&buf[(index - 2)..index]);
                                                    index = 0;
                                                    prev_index = index;

                                                    buf.clear();
                                                    lineno += 1;
                                                    if let Err(err) = options.read_line(&mut reader, &mut buf) {
                                                        if options.debug {
                                                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:1: {err}");
                                                        }
                                                        if options.strict {
                                                            return Err(Error::new(ErrorKind::IOError, err, SourceLocation::new(lineno, 1)));
                                                        }
                                                        if err.kind() == std::io::ErrorKind::InvalidData {
                                                            iter = buf.char_indices();
                                                            break;
                                                        } else {
                                                            options.set_var(env, key.as_ref(), value.as_ref());
                                                            return Ok(());
                                                        }
                                                    }

                                                    if buf.is_empty() {
                                                        if options.debug {
                                                            let line = buf.trim_end_matches('\n');
                                                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:1: syntax error: unterminated string literal: {line}");
                                                        }
                                                        if options.strict {
                                                            return Err(Error::syntax_error(lineno, 1).into());
                                                        }
                                                        options.set_var(env, key.as_ref(), value.as_ref());
                                                        return Ok(());
                                                    }

                                                    iter = buf.char_indices();
                                                }
                                            }
                                        }
                                    } else { // EOF
                                        let column = next_index + 1;
                                        if options.debug {
                                            let line = buf.trim_end_matches('\n');
                                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: unexpected end of file within escape seqeunce: {line}");
                                        }
                                        if options.strict {
                                            return Err(Error::syntax_error(lineno, column));
                                        }
                                        value.push('\\');
                                        break;
                                    }
                                }
                                '\n' => {
                                    index = next_index + 1;
                                    value.push_str(&buf[prev_index..index]);
                                    index = 0;
                                    prev_index = index;

                                    buf.clear();
                                    lineno += 1;
                                    if let Err(err) = options.read_line(&mut reader, &mut buf) {
                                        if options.debug {
                                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:1: {err}");
                                        }
                                        if options.strict {
                                            return Err(Error::with_cause(ErrorKind::IOError, err));
                                        }
                                        if err.kind() == std::io::ErrorKind::InvalidData {
                                            iter = buf.char_indices();
                                            break;
                                        } else {
                                            options.set_var(env, key.as_ref(), value.as_ref());
                                            return Ok(());
                                        }
                                    }

                                    if buf.is_empty() {
                                        if options.debug {
                                            let line = buf.trim_end_matches('\n');
                                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:1: syntax error: unterminated string literal: {line}");
                                        }
                                        if options.strict {
                                            return Err(Error::syntax_error(lineno, 1).into());
                                        }
                                        options.set_var(env, key.as_ref(), value.as_ref());
                                        return Ok(());
                                    }

                                    iter = buf.char_indices();
                                }
                                '\0' => {
                                    let column = next_index + 1;
                                    if options.debug {
                                        eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                                    }
                                    if options.strict {
                                        return Err(Error::syntax_error(lineno, column));
                                    }
                                    if index > prev_index {
                                        value.push_str(&buf[prev_index..index]);
                                    }
                                    index = next_index + 1;
                                    prev_index = index;
                                }
                                _ => {
                                    index = next_index + 1;
                                }
                            }
                        }
                    } else if ch == '#' {
                        break;
                    } else if ch == '\0' {
                        let column = index + 1;
                        if options.debug {
                            eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                        }
                        if options.strict {
                            return Err(Error::syntax_error(lineno, column));
                        }
                        let Some((next_index, next_ch)) = iter.next() else {
                            break;
                        };
                        ch = next_ch;
                        index = next_index;
                    } else {
                        let prev_index = index;

                        if ch.is_ascii_whitespace() {
                            let Some((non_ws_index, non_ws_ch)) = skipws(&mut iter) else {
                                // ignore trailing space
                                break;
                            };

                            if non_ws_ch == '#' {
                                // ignore trailing space before comment
                                break;
                            }

                            index = non_ws_index;
                            ch = non_ws_ch;

                            if ch == '"' || ch == '\'' || ch == '\0' {
                                if index > prev_index {
                                    value.push_str(&buf[prev_index..index]);
                                }
                                continue;
                            }
                        }

                        while let Some((mut next_index, mut next_ch)) = iter.next() {
                            if next_ch.is_ascii_whitespace() {
                                let Some((non_ws_index, non_ws_ch)) = skipws(&mut iter) else {
                                    // ignore trailing space
                                    index = next_index;
                                    ch = '\n';
                                    break;
                                };

                                if non_ws_ch == '#' {
                                    // ignore trailing space before comment
                                    index = next_index;
                                    ch = '\n';
                                    break;
                                }

                                next_index = non_ws_index;
                                next_ch = non_ws_ch;
                            }

                            index = next_index;
                            if next_ch == '#' {
                                ch = '\n';
                                break;
                            } else if next_ch == '"' || next_ch == '\'' || next_ch == '\0' {
                                ch = next_ch;
                                break;
                            }
                        }

                        if index > prev_index {
                            value.push_str(&buf[prev_index..index]);
                        }

                        if ch == '\n' {
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
