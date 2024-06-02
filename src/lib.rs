use std::str::CharIndices;
use std::{fs::File, io::BufReader, path::Path};

pub mod error;
pub use error::Error;
pub use error::ErrorKind;

pub mod options;
use error::SourceLocation;
pub use options::Options;

pub mod result;
pub use result::Result;

pub(crate) const DEBUG_PREFIX: &str = concat!("[", env!("CARGO_PKG_NAME"), "@", env!("CARGO_PKG_VERSION"), "][DEBUG] ");

#[inline]
pub fn load() -> Result<()> {
    let options = Options::from_env()?;
    load_from(options::config_path(), &options)
}

#[inline]
pub fn load_from(path: impl AsRef<Path>, options: &Options) -> Result<()> {
    load_from_intern(path.as_ref(), options)
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

fn load_from_intern(path: &Path, options: &Options) -> Result<()> {
    let file = File::open(path);
    let path_str = path.to_string_lossy();

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

                value.clear();
                let Some((next_index, next_ch)) = skipws(&mut iter) else {
                    options.set_var(&key, &value);
                    continue;
                };
                index = next_index;
                ch = next_ch;

                while index < buf.len() {
                    if ch == '"' || ch == '\'' {
                        let quote = ch;
                        let prev_index = index + 1;

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
                                            }
                                            '"' => {
                                                value.push('"');
                                                index = next_index + 1;
                                            }
                                            '\'' => {
                                                value.push('\'');
                                                index = next_index + 1;
                                            }
                                            'r' => {
                                                value.push('\r');
                                                index = next_index + 1;
                                            }
                                            'n' => {
                                                value.push('\n');
                                                index = next_index + 1;
                                            }
                                            't' => {
                                                value.push('\t');
                                                index = next_index + 1;
                                            }
                                            'f' => {
                                                value.push('\x0C');
                                                index = next_index + 1;
                                            }
                                            'b' => {
                                                value.push('\x08');
                                                index = next_index + 1;
                                            }
                                            '\0' => {
                                                let column = index + 1;
                                                if options.debug {
                                                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                                                }
                                                if options.strict {
                                                    return Err(Error::syntax_error(lineno, column));
                                                }
                                                value.push('\\');
                                                index = next_index;
                                            }
                                            _ => {
                                                let column = index + 1;
                                                index = next_index + 1;
                                                if options.debug {
                                                    let escseq = &buf[(index - 2)..index];
                                                    let line = buf.trim_end_matches('\n');
                                                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: illegal escape seqeunce {escseq:?}: {line}");
                                                }
                                                if options.strict {
                                                    return Err(Error::syntax_error(lineno, column));
                                                }
                                                value.push_str(&buf[(index - 2)..index]);
                                                if next_ch == '\n' {
                                                    index = 0;

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
                                                            options.set_var(&key, &value);
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
                                                        options.set_var(&key, &value);
                                                        return Ok(());
                                                    }

                                                    iter = buf.char_indices();
                                                }
                                            }
                                        }
                                    } else { // EOF
                                        let column = index + 1;
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
                                    index += 1;
                                    value.push_str(&buf[prev_index..index]);
                                    index = 0;

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
                                            options.set_var(&key, &value);
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
                                        options.set_var(&key, &value);
                                        return Ok(());
                                    }

                                    iter = buf.char_indices();
                                }
                                '\0' => {
                                    let column = index + 1;
                                    if options.debug {
                                        eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                                    }
                                    if options.strict {
                                        return Err(Error::syntax_error(lineno, column));
                                    }
                                    if index > prev_index {
                                        value.push_str(&buf[prev_index..index]);
                                    }
                                    index += 1;
                                }
                                _ => {}
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

                        while let Some((mut next_index, mut next_ch)) = iter.next() {
                            if next_ch.is_ascii_whitespace() {
                                let Some((non_ws_index, non_ws_ch)) = skipws(&mut iter) else {
                                    ch = '\n';
                                    index = next_index;
                                    break;
                                };
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

                        value.push_str(&buf[prev_index..index]);
                        if ch == '\n' {
                            break;
                        }
                    }
                }

                options.set_var(&key, &value);
            }
        }
    }

    Ok(())
}
