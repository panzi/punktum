pub mod system;
pub mod empty;
pub mod allow_list;
pub mod deny_list;

use std::{borrow::Cow, collections::HashMap, ffi::{OsStr, OsString}, hash::BuildHasher};

use crate::{options::{DEFAULT_PATH, IllegalOption, OptionType}, Dialect, Encoding, Error, ErrorKind, Result};

pub use system::{SystemEnv, SYSTEM_ENV};
pub use empty::EmptyEnv;
pub use allow_list::AllowListEnv;
pub use deny_list::DenyListEnv;

pub fn parse_bool(value: &OsStr) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Some(false)
    } else {
        None
    }
}

pub trait GetEnv {
    fn get<'a>(&'a self, key: &OsStr) -> Option<Cow<'a, OsStr>>;

    #[inline]
    fn get_config_path(&self) -> Cow<'_, OsStr> {
        self.get("DOTENV_CONFIG_PATH".as_ref())
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| {
                Cow::from(OsStr::new(DEFAULT_PATH))
            })
    }

    #[inline]
    fn get_override_env(&self) -> Result<bool> {
        self.get_bool("DOTENV_CONFIG_OVERRIDE".as_ref(), false)
    }

    #[inline]
    fn get_strict(&self) -> Result<bool> {
        self.get_bool("DOTENV_CONFIG_STRICT".as_ref(), true)
    }

    #[inline]
    fn get_debug(&self) -> Result<bool> {
        self.get_bool("DOTENV_CONFIG_DEBUG".as_ref(), false)
    }

    fn get_encoding(&self) -> Result<Encoding> {
        let encoding_key = OsStr::new("DOTENV_CONFIG_ENCODING");
        let encoding = self.get(encoding_key);
        let encoding = if let Some(encoding) = encoding {
            let Ok(encoding) = Encoding::try_from(encoding.as_ref()) else {
                return Err(Error::with_cause(
                    ErrorKind::OptionsParseError,
                    IllegalOption::new(
                        encoding_key.to_owned(),
                        encoding.into(),
                        OptionType::Encoding)));
            };
            encoding
        } else {
            Encoding::default()
        };

        Ok(encoding)
    }

    fn get_dialect(&self) -> Result<Dialect> {
        let dialect_key = OsStr::new("DOTENV_CONFIG_DIALECT");
        let dialect = self.get(dialect_key);
        let dialect = if let Some(dialect) = dialect {
            let Ok(dialect) = Dialect::try_from(dialect.as_ref()) else {
                return Err(Error::with_cause(
                    ErrorKind::OptionsParseError,
                    IllegalOption::new(
                        dialect_key.to_owned(),
                        dialect.into(),
                        OptionType::Dialect)));
            };
            dialect
        } else {
            Dialect::default()
        };

        Ok(dialect)
    }

    fn get_bool(&self, key: &OsStr, default_value: bool) -> Result<bool> {
        if let Some(value) = self.get(key) {
            let value: &OsStr = &value;
            if value.is_empty() {
                return Ok(default_value);
            }

            let Some(value) = parse_bool(value) else {
                return Err(Error::with_cause(
                    ErrorKind::OptionsParseError,
                    IllegalOption::new(
                        key.to_owned(),
                        value.into(),
                        OptionType::Bool)));
            };

            Ok(value)
        } else {
            Ok(default_value)
        }
    }
}

pub trait Env: GetEnv {
    fn set(&mut self, key: &OsStr, value: &OsStr);
    fn as_get_env(&self) -> &dyn GetEnv;
}

impl<'a> AsMut<dyn Env + 'a> for HashMap<OsString, OsString> where Self: 'a {
    #[inline]
    fn as_mut(&mut self) -> &mut (dyn Env + 'a) {
        self
    }
}

impl<'a> AsMut<dyn Env + 'a> for HashMap<String, String> where Self: 'a {
    #[inline]
    fn as_mut(&mut self) -> &mut (dyn Env + 'a) {
        self
    }
}

impl<'a> AsRef<dyn GetEnv + 'a> for HashMap<OsString, OsString> where Self: 'a {
    #[inline]
    fn as_ref(&self) -> &(dyn GetEnv + 'a) {
        self
    }
}

impl<'a> AsRef<dyn GetEnv + 'a> for HashMap<String, String> where Self: 'a {
    #[inline]
    fn as_ref(&self) -> &(dyn GetEnv + 'a) {
        self
    }
}

// XXX: Why do I need these three?
impl<T: GetEnv> GetEnv for &T {
    #[inline]
    fn get<'a>(&'a self, key: &OsStr) -> Option<Cow<'a, OsStr>> {
        (**self).get(key)
    }
}

impl<T: GetEnv> GetEnv for &mut T {
    #[inline]
    fn get<'a>(&'a self, key: &OsStr) -> Option<Cow<'a, OsStr>> {
        (**self).get(key)
    }
}

impl<T: Env> Env for &mut T where Self: GetEnv {
    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        (**self).as_get_env()
    }

    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        (**self).set(key, value);
    }
}

impl<BH: BuildHasher> GetEnv for HashMap<OsString, OsString, BH> {
    #[inline]
    fn get<'a>(&'a self, key: &OsStr) -> Option<Cow<'a, OsStr>> {
        HashMap::get(self, key).map(Cow::from)
    }
}

impl<BH: BuildHasher> Env for HashMap<OsString, OsString, BH> {
    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        self.insert(key.to_os_string(), value.to_os_string());
    }

    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        self
    }
}

impl<BH: BuildHasher> GetEnv for HashMap<String, String, BH> {
    #[inline]
    fn get<'a>(&'a self, key: &OsStr) -> Option<Cow<'a, OsStr>> {
        HashMap::get(self, key.to_string_lossy().as_ref()).map(|value| {
            let value: &OsStr = value.as_ref();
            Cow::from(value)
        })
    }
}

impl<BH: BuildHasher> Env for HashMap<String, String, BH> {
    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        self.insert(key.to_string_lossy().into_owned(), value.to_string_lossy().into_owned());
    }

    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        self
    }
}
