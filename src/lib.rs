use std::str::CharIndices;
use std::{fs::File, io::BufReader, path::Path};

pub mod error;
pub use error::Error;
pub use error::ErrorKind;

pub mod options;
pub use options::Options;

pub mod result;
pub use result::Result;

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
                eprintln!("{path_str}: {err}");
            }
            if options.strict {
                return Err(Error::new(ErrorKind::IOError, err));
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
                        eprintln!("{path_str}:{lineno}:1: {err}");
                    }
                    if options.strict {
                        return Err(Error::new(ErrorKind::IOError, err));
                    }
                    return Ok(());
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

                let mut prev_index = index;

                let Some((mut index, mut ch)) = skip_word(&mut iter) else {
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        let column = prev_index + 1;
                        eprintln!("{path_str}:{lineno}:{column}: syntax error: {line}");
                    }
                    if options.strict {
                        return Err(ErrorKind::SyntaxError.into());
                    }
                    continue;
                };

                key.clear();
                key.push_str(&buf[prev_index..index]);

                if key.is_empty() {
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        let column = index + 1;
                        eprintln!("{path_str}:{lineno}:{column}: syntax error: unexpected {ch:?}: {line}");
                    }
                    if options.strict {
                        return Err(ErrorKind::SyntaxError.into());
                    }
                    continue;
                }

                if ch != '=' {
                    if !ch.is_ascii_whitespace() {
                        if options.debug {
                            let line = buf.trim_end_matches('\n');
                            let column = index + 1;
                            eprintln!("{path_str}:{lineno}:{column}: syntax error: unexpected {ch:?}: {line}");
                        }
                        if options.strict {
                            return Err(ErrorKind::SyntaxError.into());
                        }
                        continue;
                    }

                    let Some((next_index, next_ch)) = skipws(&mut iter) else {
                        if options.debug {
                            let line = buf.trim_end_matches('\n');
                            let column = index + 1;
                            eprintln!("{path_str}:{lineno}:{column}: syntax error: unexpected end of line, expected '=': {line}");
                        }
                        if options.strict {
                            return Err(ErrorKind::SyntaxError.into());
                        }
                        continue;
                    };
                    index = next_index;
                    ch = next_ch;
                }

                if ch != '=' {
                    if options.debug {
                        let line = buf.trim_end_matches('\n');
                        let column = index + 1;
                        eprintln!("{path_str}:{lineno}:{column}: syntax error: expected '=', actual {ch:?}: {line}");
                    }
                    if options.strict {
                        return Err(ErrorKind::SyntaxError.into());
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

                if ch == '"' {
                    prev_index = index + 1;

                    loop {
                        let Some((index, ch)) = iter.next() else {
                            if options.debug {
                                let line = buf.trim_end_matches('\n');
                                let column = prev_index + 1;
                                eprintln!("{path_str}:{lineno}:{column}: syntax error: unterminated string literal: {line}");
                            }
                            if options.strict {
                                return Err(ErrorKind::SyntaxError.into());
                            }
                            value.push_str(&buf[prev_index..]);
                            break;
                        };

                        match ch {
                            '\\' => {
                                if index > prev_index {
                                    value.push_str(&buf[prev_index..index]);
                                }

                                if let Some((index, ch)) = iter.next() {
                                    match ch {
                                        '\\' => {
                                            value.push('\\');
                                            prev_index = index + 1;
                                        }
                                        '"' => {
                                            value.push('"');
                                            prev_index = index + 1;
                                        }
                                        'r' => {
                                            value.push('\r');
                                            prev_index = index + 1;
                                        }
                                        'n' => {
                                            value.push('\n');
                                            prev_index = index + 1;
                                        }
                                        't' => {
                                            value.push('\t');
                                            prev_index = index + 1;
                                        }
                                        'f' => {
                                            value.push('\x0C');
                                            prev_index = index + 1;
                                        }
                                        'b' => {
                                            value.push('\x08');
                                            prev_index = index + 1;
                                        }
                                        '\0' => {
                                            if options.debug {
                                                let column = index + 1;
                                                eprintln!("{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                                            }
                                            if options.strict {
                                                return Err(ErrorKind::SyntaxError.into());
                                            }
                                            value.push('\\');
                                            prev_index = index + 1;
                                        }
                                        _ => {
                                            if options.debug {
                                                let line = buf.trim_end_matches('\n');
                                                let column = index + 1;
                                                eprintln!("{path_str}:{lineno}:{column}: syntax error: illegal escape seqeunce: {line}");
                                            }
                                            if options.strict {
                                                return Err(ErrorKind::SyntaxError.into());
                                            }
                                            value.push_str(&buf[(index - 1)..(index + 1)]);
                                            if ch == '\n' {
                                                prev_index = 0;

                                                buf.clear();
                                                lineno += 1;
                                                if let Err(err) = options.read_line(&mut reader, &mut buf) {
                                                    if options.debug {
                                                        eprintln!("{path_str}:{lineno}:1: {err}");
                                                    }
                                                    if options.strict {
                                                        return Err(Error::new(ErrorKind::IOError, err));
                                                    }
                                                    options.set_var(&key, &value);
                                                    return Ok(());
                                                }

                                                if buf.is_empty() {
                                                    if options.debug {
                                                        let line = buf.trim_end_matches('\n');
                                                        eprintln!("{path_str}:{lineno}:1: syntax error: unterminated string literal: {line}");
                                                    }
                                                    if options.strict {
                                                        return Err(ErrorKind::SyntaxError.into());
                                                    }
                                                    options.set_var(&key, &value);
                                                    return Ok(());
                                                }

                                                iter = buf.char_indices();
                                            } else {
                                                prev_index = index + 1;
                                            }
                                        }
                                    }
                                } else {
                                    if options.debug {
                                        let line = buf.trim_end_matches('\n');
                                        let column = index + 1;
                                        eprintln!("{path_str}:{lineno}:{column}: syntax error: unexpected end of line within escape seqeunce: {line}");
                                    }
                                    if options.strict {
                                        return Err(ErrorKind::SyntaxError.into());
                                    }
                                    value.push('\\');
                                    prev_index = index;
                                }
                            }
                            '\n' => {
                                let index = index + 1;
                                value.push_str(&buf[prev_index..index]);
                                prev_index = 0;

                                buf.clear();
                                lineno += 1;
                                if let Err(err) = options.read_line(&mut reader, &mut buf) {
                                    if options.debug {
                                        eprintln!("{path_str}:{lineno}:1: {err}");
                                    }
                                    if options.strict {
                                        return Err(Error::new(ErrorKind::IOError, err));
                                    }
                                    options.set_var(&key, &value);
                                    return Ok(());
                                }

                                if buf.is_empty() {
                                    if options.debug {
                                        let line = buf.trim_end_matches('\n');
                                        eprintln!("{path_str}:{lineno}:1: syntax error: unterminated string literal: {line}");
                                    }
                                    if options.strict {
                                        return Err(ErrorKind::SyntaxError.into());
                                    }
                                    options.set_var(&key, &value);
                                    return Ok(());
                                }

                                iter = buf.char_indices();
                            }
                            '"' => {
                                if index > prev_index {
                                    value.push_str(&buf[prev_index..index]);
                                }
                                break;
                            }
                            '\0' => {
                                if options.debug {
                                    let column = index + 1;
                                    eprintln!("{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                                }
                                if options.strict {
                                    return Err(ErrorKind::SyntaxError.into());
                                }
                                if index > prev_index {
                                    value.push_str(&buf[prev_index..index]);
                                }
                                prev_index = index + 1;
                            }
                            _ => {}
                        }
                    }

                    if let Some((next_index, next_ch)) = iter.next() {
                        if !next_ch.is_ascii_whitespace() {
                            if options.debug {
                                let line = buf.trim_end_matches('\n');
                                let column = next_index + 1;
                                eprintln!("{path_str}:{lineno}:{column}: syntax error: unexpected {next_ch:?}: {line}");
                            }
                            if options.strict {
                                return Err(ErrorKind::SyntaxError.into());
                            }
                        }

                        if let Some((_, next_ch)) = skipws(&mut iter) {
                            if next_ch != '#' {
                                if options.debug {
                                    let line = buf.trim_end_matches('\n');
                                    let column = next_index + 1;
                                    eprintln!("{path_str}:{lineno}:{column}: syntax error: unexpected {next_ch:?}: {line}");
                                }
                                if options.strict {
                                    return Err(ErrorKind::SyntaxError.into());
                                }
                            }
                        }
                    }
                } else if ch != '#' {
                    if ch == '\0' {
                        if options.debug {
                            let column = index + 1;
                            eprintln!("{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                        }
                        if options.strict {
                            return Err(ErrorKind::SyntaxError.into());
                        }
                        index += 1;
                    }

                    prev_index = index;

                    while let Some((mut next_index, mut ch)) = iter.next() {
                        if ch.is_ascii_whitespace() {
                            index = next_index;
                            let Some((non_ws_index, next_ch)) = skipws(&mut iter) else {
                                break;
                            };
                            next_index = non_ws_index;
                            ch = next_ch;

                            if ch == '#' {
                                break;
                            }
                        }

                        if ch == '\0' {
                            if options.debug {
                                let column = next_index + 1;
                                eprintln!("{path_str}:{lineno}:{column}: syntax error: illegal null byte: {buf:?}");
                            }
                            if options.strict {
                                return Err(ErrorKind::SyntaxError.into());
                            }
                            if next_index > prev_index {
                                value.push_str(&buf[prev_index..next_index]);
                            }
                            index = next_index + 1;
                            prev_index = index;
                        } else {
                            index = next_index;
                        }
                    }

                    value.push_str(&buf[prev_index..index]);
                }

                options.set_var(&key, &value);
            }
        }
    }

    Ok(())
}
