use std::{ffi::{OsStr, OsString}, fs::File, io::{BufRead, BufReader}, path::Path};

use crate::{env::{GetEnv, SystemEnv}, Env, Error, ErrorKind, Result, DEBUG_PREFIX};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Encoding {
    ASCII,
    /// aka ISO-8859-1
    Latin1,
    UTF8,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct IllegalEncoding();

impl std::fmt::Display for IllegalEncoding {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "IllegalEncoding".fmt(f)
    }
}

impl std::error::Error for IllegalEncoding {}

impl TryFrom<&OsStr> for Encoding {
    type Error = IllegalEncoding;

    fn try_from(value: &OsStr) -> std::result::Result<Self, Self::Error> {
        if value.eq_ignore_ascii_case("ascii") {
            Ok(Encoding::ASCII)
        } else if value.eq_ignore_ascii_case("latin1") || value.eq_ignore_ascii_case("iso-8859-1") {
            Ok(Encoding::Latin1)
        } else if value.eq_ignore_ascii_case("utf-8") || value.eq_ignore_ascii_case("utf8") {
            Ok(Encoding::UTF8)
        } else {
            Err(IllegalEncoding())
        }
    }
}

impl std::str::FromStr for Encoding {
    type Err = IllegalEncoding;

    #[inline]
    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Encoding::try_from(value.as_ref())
    }
}

impl Default for Encoding {
    #[inline]
    fn default() -> Self {
        Encoding::UTF8
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Options<P=OsString>
where P: AsRef<Path> + Clone {
    /// Override existing environment variables.
    pub override_env: bool,

    /// Error on IO or parser errors.
    pub strict: bool,

    /// Log IO and parser errors.
    pub debug: bool,

    pub encoding: Encoding,

    pub path: P,
}

#[inline]
pub fn default_path() -> OsString {
    OsString::from(".env")
}

impl Default for Options {
    #[inline]
    fn default() -> Self {
        Self {
            override_env: false,
            strict: true,
            debug: false,
            encoding: Encoding::default(),
            path: default_path(),
        }
    }
}

impl Options {
    pub fn try_from<E: GetEnv>(env: &E) -> Result<Self> {
        let override_env = env.get_override_env()?;
        let strict = env.get_strict()?;
        let debug = env.get_debug()?;
        let encoding = env.get_encoding()?;
        let path = env.get_config_path();

        Ok(Self { override_env, strict, debug, encoding, path })
    }

    #[inline]
    pub fn try_from_env() -> Result<Self> {
        Self::try_from(&SystemEnv::get())
    }
}

impl<P> Options<P>
where P: AsRef<Path> + Clone {
    #[inline]
    pub fn config(&self) -> Result<()> {
        crate::config_with(&mut SystemEnv::get(), self)
    }

    #[inline]
    pub fn config_env<E: Env>(&self, env: &mut E) -> Result<()> {
        crate::config_with(env, self)
    }

    #[inline]
    pub(crate) fn set_var(&self, env: &mut impl Env, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        let key = key.as_ref();
        if self.override_env {
            env.set(key, value);
        } else if env.get(key).is_some() {
            if self.debug {
                eprintln!("{DEBUG_PREFIX}{key:?} is already defined and was NOT overwritten");
            }
        } else {
            env.set(key, value);
        }
    }

    pub(crate) fn read_line(&self, reader: &mut BufReader<File>, line: &mut String) -> std::io::Result<usize> {
        match self.encoding {
            Encoding::UTF8 => reader.read_line(line),
            Encoding::ASCII => {
                let mut buf = Vec::new();
                let num_bytes = reader.read_until('\n' as u8, &mut buf)?;

                for byte in buf.iter().cloned() {
                    if byte > 127 {
                        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
                    }
                }

                line.extend(buf.into_iter().map(|byte| byte as char));

                Ok(num_bytes)
            },
            Encoding::Latin1 => {
                let mut buf = Vec::new();
                let num_bytes = reader.read_until('\n' as u8, &mut buf)?;

                line.extend(buf.into_iter().map(|byte| byte as char));

                Ok(num_bytes)
            }
        }
    }
}

#[inline]
pub(crate) fn getenv_bool<E: GetEnv>(env: &E, key: impl AsRef<OsStr>, default_value: bool) -> Result<bool> {
    getenv_bool_intern(env, key.as_ref(), default_value)
}

fn getenv_bool_intern<E: GetEnv>(env: &E, key: &OsStr, default_value: bool) -> Result<bool> {
    if let Some(value) = env.get(key) {
        if value.eq_ignore_ascii_case("true") || value.eq("1") {
            Ok(true)
        } else if value.eq_ignore_ascii_case("false") || value.is_empty() || value.eq("0") {
            Ok(false)
        } else {
            Err(Error::with_cause(
                ErrorKind::OptionsParseError,
                IllegalOption::new(key.to_owned(), value, OptionType::Bool)))
        }
    } else {
        Ok(default_value)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OptionType {
    Bool,
    Encoding,
}

impl std::fmt::Display for OptionType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug)]
pub struct IllegalOption {
    name: OsString,
    value: OsString,
    option_type: OptionType,
}

impl IllegalOption {
    #[inline]
    pub fn new(name: OsString, value: OsString, option_type: OptionType) -> Self {
        Self { name, value, option_type }
    }

    #[inline]
    pub fn name(&self) -> &OsStr {
        &self.name
    }

    #[inline]
    pub fn value(&self) -> &OsStr {
        &self.value
    }

    #[inline]
    pub fn option_type(&self) -> OptionType {
        self.option_type
    }
}

impl std::fmt::Display for IllegalOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} option has illegal value: {:?}={:?}", self.option_type, self.name, self.value)
    }
}

impl std::error::Error for IllegalOption {}

#[derive(Debug, PartialEq, Clone)]
pub struct Builder<P=OsString>
where P: AsRef<Path> + Clone {
    options: Options<P>,
}

impl Default for Builder {
    #[inline]
    fn default() -> Self {
        Self { options: Options::default() }
    }
}

impl Builder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn try_from<E: GetEnv>(env: &E) -> Result<Self> {
        let options = Options::try_from(env)?;
        Ok(Self { options })
    }

    #[inline]
    pub fn try_from_env() -> Result<Self> {
        let options = Options::try_from_env()?;
        Ok(Self { options })
    }
}

impl<P> Builder<P>
where P: AsRef<Path> + Clone {
    #[inline]
    pub fn override_env(mut self, value: bool) -> Self {
        self.options.override_env = value;
        self
    }

    #[inline]
    pub fn strict(mut self, value: bool) -> Self {
        self.options.strict = value;
        self
    }

    #[inline]
    pub fn debug(mut self, value: bool) -> Self {
        self.options.debug = value;
        self
    }

    #[inline]
    pub fn encoding(mut self, value: Encoding) -> Self {
        self.options.encoding = value;
        self
    }

    pub fn path<NewP>(&self, value: NewP) -> Builder<NewP>
    where NewP: AsRef<Path> + Clone {
        Builder {
            options: Options {
                override_env: self.options.override_env,
                debug: self.options.debug,
                strict: self.options.strict,
                encoding: self.options.encoding,
                path: value,
            }
        }
    }

    #[inline]
    pub fn options(&self) -> &Options<P> {
        &self.options
    }

    #[inline]
    pub fn options_mut(&mut self) -> &mut Options<P> {
        &mut self.options
    }

    #[inline]
    pub fn into_options(self) -> Options<P> {
        self.options
    }

    #[inline]
    pub fn config(&self) -> Result<()> {
        self.options.config()
    }

    #[inline]
    pub fn config_env<E: Env>(&self, env: &mut E) -> Result<()> {
        self.options.config_env(env)
    }
}

impl<P> From<Options<P>> for Builder<P>
where P: AsRef<Path> + Clone {
    #[inline]
    fn from(options: Options<P>) -> Self {
        Self {
            options
        }
    }
}

impl<P> From<&Options<P>> for Builder<P>
where P: AsRef<Path> + Clone {
    #[inline]
    fn from(options: &Options<P>) -> Self {
        Self {
            options: options.clone()
        }
    }
}

impl<P> From<Builder<P>> for Options<P>
where P: AsRef<Path> + Clone {
    #[inline]
    fn from(value: Builder<P>) -> Self {
        value.into_options()
    }
}
