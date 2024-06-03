use std::{collections::HashMap, ffi::{OsStr, OsString}, hash::BuildHasher, sync::Mutex};

use crate::{options::{default_path, Encoding, IllegalOption, OptionType}, Error, ErrorKind, Result};

pub trait GetEnv {
    fn get(&self, key: &OsStr) -> Option<OsString>;

    #[inline]
    fn get_config_path(&self) -> OsString {
        self.get("DOTENV_CONFIG_PATH".as_ref())
            .filter(|path| !path.is_empty())
            .unwrap_or_else(default_path)
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

        Ok(encoding)
    }

    fn get_bool(&self, key: &OsStr, default_value: bool) -> Result<bool> {
        let key = key.as_ref();
        if let Some(value) = self.get(key) {
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
}

pub trait Env: GetEnv {
    fn set(&mut self, key: &OsStr, value: &OsStr);
}

/// Accessing the environment is unsafe (not thread safe), but the std::env::*
/// functions aren't marked as unsafe. This mutex doesn't really fix the issue
/// since it only applies to code accessing the environment through
/// [`SystemEnv`].
static MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct SystemEnv();

impl SystemEnv {
    #[inline]
    pub fn get() -> Self {
        Self ()
    }

    pub fn hash_map() -> HashMap<OsString, OsString> {
        let mut vars = HashMap::new();
        let _lock = MUTEX.lock();

        for (key, value) in std::env::vars_os() {
            vars.insert(key, value);
        }

        vars
    }

    pub fn hash_map_lossy() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        let _lock = MUTEX.lock();

        for (key, value) in std::env::vars_os() {
            vars.insert(
                key.to_string_lossy().to_string(),
                value.to_string_lossy().to_string());
        }

        vars
    }

    #[inline]
    pub fn to_hash_map(&self) -> HashMap<OsString, OsString> {
        Self::hash_map()
    }

    #[inline]
    pub fn to_hash_map_lossy(&self) -> HashMap<String, String> {
        Self::hash_map_lossy()
    }
}

impl AsRef<SystemEnv> for SystemEnv {
    #[inline]
    fn as_ref(&self) -> &SystemEnv {
        self
    }
}

impl AsMut<SystemEnv> for SystemEnv {
    #[inline]
    fn as_mut(&mut self) -> &mut SystemEnv {
        self
    }
}

impl GetEnv for SystemEnv {
    fn get(&self, key: &OsStr) -> Option<OsString> {
        let _lock = MUTEX.lock();

        std::env::var_os(key)
    }
}

impl Env for SystemEnv {
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        let _lock = MUTEX.lock();

        std::env::set_var(key, value);
    }
}

impl<BH: BuildHasher> GetEnv for HashMap<OsString, OsString, BH> {
    #[inline]
    fn get(&self, key: &OsStr) -> Option<OsString> {
        HashMap::get(self, key).map(|value| value.to_os_string())
    }
}

impl<BH: BuildHasher> Env for HashMap<OsString, OsString, BH> {
    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        self.insert(key.to_os_string(), value.to_os_string());
    }
}

impl<BH: BuildHasher> GetEnv for HashMap<String, String, BH> {
    #[inline]
    fn get(&self, key: &OsStr) -> Option<OsString> {
        HashMap::get(self, key.to_string_lossy().as_ref()).map(|value| value.into())
    }
}

impl<BH: BuildHasher> Env for HashMap<String, String, BH> {
    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        self.insert(key.to_string_lossy().into_owned(), value.to_string_lossy().into_owned());
    }
}

impl From<SystemEnv> for HashMap<OsString, OsString> {
    #[inline]
    fn from(_value: SystemEnv) -> Self {
        SystemEnv::hash_map()
    }
}

impl From<&SystemEnv> for HashMap<OsString, OsString> {
    #[inline]
    fn from(_value: &SystemEnv) -> Self {
        SystemEnv::hash_map()
    }
}

impl From<SystemEnv> for HashMap<String, String> {
    #[inline]
    fn from(_value: SystemEnv) -> Self {
        SystemEnv::hash_map_lossy()
    }
}

impl From<&SystemEnv> for HashMap<String, String> {
    #[inline]
    fn from(_value: &SystemEnv) -> Self {
        SystemEnv::hash_map_lossy()
    }
}
