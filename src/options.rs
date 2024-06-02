use std::{ffi::{OsStr, OsString}, fs::File, io::{BufRead, BufReader}, path::Path};

use crate::{Error, ErrorKind, Result};

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Options {
    /// Override existing environment variables.
    pub override_env: bool,

    /// Error on IO or parser errors.
    pub strict: bool,

    /// Log IO and parser errors.
    pub debug: bool,

    pub encoding: Encoding,
}

impl Default for Options {
    #[inline]
    fn default() -> Self {
        Self {
            override_env: false,
            strict: true,
            debug: false,
            encoding: Encoding::default(),
        }
    }
}

impl Options {
    pub fn from_env() -> Result<Self> {
        let override_env = getenv_bool("DOTENV_CONFIG_OVERRIDE", false)?;
        let strict = getenv_bool("DOTENV_CONFIG_STRICT", true)?; // extension!
        let debug = getenv_bool("DOTENV_CONFIG_DEBUG", false)?;

        let encoding_key = OsStr::new("DOTENV_CONFIG_ENCODING");
        let encoding = std::env::var_os(encoding_key);
        let encoding = if let Some(encoding) = encoding {
            let Ok(encoding) = Encoding::try_from(encoding.as_os_str()) else {
                return Err(Error::with_cause(
                    ErrorKind::OptionsParseError,
                    IllegalOption::new(
                        encoding_key.to_owned(),
                        encoding,
                        OptionType::Encoding)));
            };
            encoding
        } else {
            Encoding::default()
        };

        Ok(Self { override_env, strict, debug, encoding })
    }

    #[inline]
    pub(crate) fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(&self, key: K, value: V) {
        let key = key.as_ref();
        if self.override_env || std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }

    #[inline]
    pub fn load(&self) -> Result<()> {
        crate::load_from(config_path(), self)
    }

    #[inline]
    pub fn load_from(&self, path: impl AsRef<Path>) -> Result<()> {
        crate::load_from(path, self)
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
pub fn config_path() -> OsString {
    std::env::var_os("DOTENV_CONFIG_PATH").unwrap_or(OsString::from(".env"))
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

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Builder {
    options: Options
}

impl Builder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_env() -> Result<Self> {
        let options = Options::from_env()?;
        Ok(Self { options })
    }

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

    #[inline]
    pub fn options(&self) -> &Options {
        &self.options
    }

    #[inline]
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    #[inline]
    pub fn into_options(self) -> Options {
        self.options
    }

    #[inline]
    pub fn load(&self) -> Result<()> {
        self.options.load()
    }

    #[inline]
    pub fn load_from(&self, path: impl AsRef<Path>) -> Result<()> {
        self.options.load_from(path)
    }
}
