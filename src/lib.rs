use std::str::CharIndices;
use std::{ffi::OsStr, fs::File, io::BufReader, path::Path};
use std::io::BufRead;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ErrorKind {
    OptionsParseError,
    IOError,
    SyntaxError,
    ExecError,
}

impl std::fmt::Display for ErrorKind {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug)]
pub struct Error {
    cause: Option<Box<dyn std::error::Error>>,
    kind: ErrorKind,
}

impl Error {
    #[inline]
    pub fn new(kind: ErrorKind) -> Self {
        Self { cause: None, kind }
    }

    #[inline]
    pub fn with_cause(kind: ErrorKind, cause: Box<dyn std::error::Error>) -> Self {
        Self { cause: Some(cause), kind }
    }

    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl std::fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

impl std::error::Error for Error {
    #[inline]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        if let Some(cause) = &self.cause {
            Some(cause.as_ref())
        } else {
            None
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::with_cause(ErrorKind::IOError, Box::new(value))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Options {
    /// Override existing environment variables.
    pub override_env: bool,

    /// Error on IO or parser errors.
    pub strict: bool,

    /// Log IO and parser errors.
    pub debug: bool,
}

#[inline]
fn getenv_bool(key: impl AsRef<OsStr>, default_value: bool) -> Result<bool> {
    getenv_bool_intern(key.as_ref(), default_value)
}

fn getenv_bool_intern(key: &OsStr, default_value: bool) -> Result<bool> {
    if let Some(value) = std::env::var_os(key) {
        if value.eq_ignore_ascii_case("true") || value.eq("1") {
            Ok(true)
        } else if value.eq_ignore_ascii_case("false") || value.is_empty() || value.eq("0") {
            Ok(false)
        } else {
            eprintln!("illegal value for environment variable {key:?}={value:?}");
            Err(Error::new(ErrorKind::OptionsParseError))
        }
    } else {
        Ok(default_value)
    }
}

impl Options {
    pub fn from_env() -> Result<Self> {
        let override_env = getenv_bool("DOTENV_CONFIG_OVERRIDE", false)?;
        let strict = getenv_bool("DOTENV_CONFIG_STRICT", false)?; // extension!
        let debug = getenv_bool("DOTENV_CONFIG_DEBUG", false)?;

        Ok(Self { override_env, strict, debug })
    }

    #[inline]
    fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(&self, key: K, value: V) {
        let key = key.as_ref();
        if self.override_env || std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }
}

pub fn load() -> Result<()> {
    let path = std::env::var_os("DOTENV_CONFIG_PATH");
    let options = Options::from_env()?;

    let path = if let Some(path) = &path {
        path
    } else {
        OsStr::new(".env")
    };

    load_from(path, options)
}

#[inline]
pub fn load_from(path: impl AsRef<Path>, options: Options) -> Result<()> {
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

fn load_from_intern(path: &Path, options: Options) -> Result<()> {
    let file = File::open(path);
    let path_str = path.to_string_lossy();

    match file {
        Err(err) => {
            if options.debug {
                eprintln!("{path_str}: {err}");
            }
            if options.strict {
                return Err(Error::with_cause(ErrorKind::IOError, Box::new(err)));
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
                if let Err(err) = reader.read_line(&mut buf) {
                    if options.debug {
                        eprintln!("{path_str}:{lineno}: {err}");
                    }
                    if options.strict {
                        return Err(Error::with_cause(ErrorKind::IOError, Box::new(err)));
                    }
                    return Ok(());
                }
                lineno += 1;

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
                        eprintln!("{path_str}:{lineno}: syntax error: {buf}");
                    }
                    if options.strict {
                        return Err(Error::new(ErrorKind::SyntaxError));
                    }
                    continue;
                };

                key.clear();
                key.push_str(&buf[prev_index..index]);

                if key.is_empty() {
                    if options.debug {
                        eprintln!("{path_str}:{lineno}: syntax error: {buf}");
                    }
                    if options.strict {
                        return Err(Error::new(ErrorKind::SyntaxError));
                    }
                    continue;
                }

                if ch != '=' {
                    if !ch.is_ascii_whitespace() {
                        if options.debug {
                            eprintln!("{path_str}:{lineno}: syntax error: {buf}");
                        }
                        if options.strict {
                            return Err(Error::new(ErrorKind::SyntaxError));
                        }
                        continue;
                    }

                    let Some((_, next_ch)) = skipws(&mut iter) else {
                        if options.debug {
                            eprintln!("{path_str}:{lineno}: syntax error: unexpected end of line, expected '=': {buf}");
                        }
                        if options.strict {
                            return Err(Error::new(ErrorKind::SyntaxError));
                        }
                        continue;
                    };
                    ch = next_ch;
                }

                if ch != '=' {
                    if options.debug {
                        eprintln!("{path_str}:{lineno}: syntax error: expected '=', actual {ch:?}: {buf}");
                    }
                    if options.strict {
                        return Err(Error::new(ErrorKind::SyntaxError));
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
                                eprintln!("{path_str}:{lineno}: syntax error: unterminated string literal: {buf}");
                            }
                            if options.strict {
                                return Err(Error::new(ErrorKind::SyntaxError));
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
                                                eprintln!("{path_str}:{lineno}: syntax error: illegal null byte: {buf}");
                                            }
                                            if options.strict {
                                                return Err(Error::new(ErrorKind::SyntaxError));
                                            }
                                            value.push('\\');
                                            prev_index = index + 1;
                                        }
                                        _ => {
                                            if options.debug {
                                                eprintln!("{path_str}:{lineno}: syntax error: illegal escape seqeunce: {buf}");
                                            }
                                            if options.strict {
                                                return Err(Error::new(ErrorKind::SyntaxError));
                                            }
                                            value.push(ch);
                                            prev_index = index + 1;
                                        }
                                    }
                                } else {
                                    if options.debug {
                                        eprintln!("{path_str}:{lineno}: syntax error: illegal escape seqeunce: {buf}");
                                    }
                                    if options.strict {
                                        return Err(Error::new(ErrorKind::SyntaxError));
                                    }
                                    prev_index = index;
                                }
                            }
                            '\n' => {
                                let index = index + 1;
                                value.push_str(&buf[prev_index..index]);
                                prev_index = 0;

                                buf.clear();
                                if let Err(err) = reader.read_line(&mut buf) {
                                    if options.debug {
                                        eprintln!("{path_str}:{lineno}: {err}");
                                    }
                                    if options.strict {
                                        return Err(Error::with_cause(ErrorKind::IOError, Box::new(err)));
                                    }
                                    options.set_var(&key, &value);
                                    return Ok(());
                                }
                                lineno += 1;

                                if buf.is_empty() {
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
                                    eprintln!("{path_str}:{lineno}: syntax error: illegal null byte: {buf}");
                                }
                                if options.strict {
                                    return Err(Error::new(ErrorKind::SyntaxError));
                                }
                                if index > prev_index {
                                    value.push_str(&buf[prev_index..index]);
                                }
                                prev_index = index + 1;
                            }
                            _ => {}
                        }
                    }

                    if let Some((_, next_ch)) = iter.next() {
                        if !next_ch.is_ascii_whitespace() {
                            if options.debug {
                                eprintln!("{path_str}:{lineno}: syntax error: unexpected {next_ch:?}: {buf}");
                            }
                            if options.strict {
                                return Err(Error::new(ErrorKind::SyntaxError));
                            }
                        }

                        if let Some((_, next_ch)) = skipws(&mut iter) {
                            if next_ch != '#' {
                                if options.debug {
                                    eprintln!("{path_str}:{lineno}: syntax error: unexpected {next_ch:?}: {buf}");
                                }
                                if options.strict {
                                    return Err(Error::new(ErrorKind::SyntaxError));
                                }
                            }
                        }
                    }
                } else if ch != '#' {
                    if ch == '\0' {
                        if options.debug {
                            eprintln!("{path_str}:{lineno}: syntax error: illegal null byte: {buf}");
                        }
                        if options.strict {
                            return Err(Error::new(ErrorKind::SyntaxError));
                        }
                        index += 1;
                    }

                    prev_index = index;

                    while let Some((mut next_index, mut ch)) = iter.next() {
                        if ch.is_ascii_whitespace() {
                            index = next_index;
                            let Some((nonws_index, next_ch)) = skipws(&mut iter) else {
                                break;
                            };
                            next_index = nonws_index;
                            ch = next_ch;

                            if ch == '#' {
                                break;
                            }
                        }

                        if ch == '\0' {
                            if options.debug {
                                eprintln!("{path_str}:{lineno}: syntax error: illegal null byte: {buf}");
                            }
                            if options.strict {
                                return Err(Error::new(ErrorKind::SyntaxError));
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
