use std::{borrow::Cow, collections::{HashMap, HashSet}, ffi::{OsStr, OsString}, hash::BuildHasher, sync::Mutex};

use crate::{options::{DEFAULT_PATH, IllegalOption, OptionType}, Dialect, Encoding, Error, ErrorKind, Result};

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

pub fn parse_bool(value: &OsStr) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Some(false)
    } else {
        None
    }
}

pub trait Env: GetEnv {
    fn set(&mut self, key: &OsStr, value: &OsStr);
    fn as_get_env(&self) -> &dyn GetEnv;
}

/// Accessing the environment is unsafe (not thread safe), but the std::env::*
/// functions aren't marked as unsafe. This mutex doesn't really fix the issue
/// since it only applies to code accessing the environment through
/// [`SystemEnv`].
#[cfg(not(target_family = "windows"))]
static MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy)]
pub struct SystemEnv();

pub const SYSTEM_ENV: SystemEnv = SystemEnv();

impl Default for SystemEnv {
    #[inline]
    fn default() -> Self {
        Self()
    }
}

impl SystemEnv {
    #[inline]
    pub fn new() -> Self {
        Self()
    }

    pub fn hash_map() -> HashMap<OsString, OsString> {
        let mut vars = HashMap::new();
        #[cfg(not(target_family = "windows"))]
        let _lock = MUTEX.lock();

        for (key, value) in std::env::vars_os() {
            vars.insert(key, value);
        }

        vars
    }

    pub fn hash_map_lossy() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        #[cfg(not(target_family = "windows"))]
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
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl GetEnv for SystemEnv {
    fn get<'a>(&'a self, key: &OsStr) -> Option<Cow<'a, OsStr>> {
        #[cfg(not(target_family = "windows"))]
        let _lock = MUTEX.lock();

        std::env::var_os(key).map(Cow::from)
    }
}

impl Env for SystemEnv {
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        #[cfg(not(target_family = "windows"))]
        let _lock = MUTEX.lock();

        std::env::set_var(key, value);
    }

    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EmptyEnv();

impl Default for EmptyEnv {
    #[inline]
    fn default() -> Self {
        Self()
    }
}

impl EmptyEnv {
    #[inline]
    pub fn new() -> Self {
        Self()
    }
}

impl GetEnv for EmptyEnv {
    #[inline]
    fn get<'a>(&'a self, _key: &OsStr) -> Option<Cow<'a, OsStr>> {
        None
    }
}

impl AsRef<EmptyEnv> for EmptyEnv {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsMut<EmptyEnv> for EmptyEnv {
    #[inline]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl<'a> AsRef<dyn GetEnv + 'a> for EmptyEnv where Self: 'a {
    #[inline]
    fn as_ref(&self) -> &(dyn GetEnv + 'a) {
        self
    }
}

impl<'a> AsRef<dyn GetEnv + 'a> for SystemEnv where Self: 'a {
    #[inline]
    fn as_ref(&self) -> &(dyn GetEnv + 'a) {
        self
    }
}

impl<'a> AsMut<dyn Env + 'a> for SystemEnv where Self: 'a {
    #[inline]
    fn as_mut(&mut self) -> &mut (dyn Env + 'a) {
        self
    }
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

pub struct DenyListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    env: E,
    deny_list: HashSet<&'a OsStr>,
}

impl<'a, E> DenyListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    #[inline]
    pub fn new(env: E, deny_list: HashSet<&'a OsStr>) -> Self {
        Self { env, deny_list }
    }

    #[inline]
    pub fn from_slice(env: E, deny_list: &'a [impl AsRef<OsStr>]) -> Self {
        Self {
            env,
            deny_list: deny_list.iter().map(|key| (*key).as_ref()).collect()
        }
    }

    #[inline]
    pub fn from_iter(env: E, deny_list: impl Iterator<Item=&'a OsStr>) -> Self {
        Self {
            env,
            deny_list: deny_list.collect()
        }
    }

    #[inline]
    pub fn env(&self) -> &E {
        &self.env
    }

    #[inline]
    pub fn deny_list(&self) -> &HashSet<&'a OsStr> {
        &self.deny_list
    }

    #[inline]
    pub fn into_env(self) -> E {
        self.env
    }
}

impl<'a, E> GetEnv for DenyListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    #[inline]
    fn get<'b>(&'b self, key: &OsStr) -> Option<Cow<'b, OsStr>> {
        self.env.as_ref().get(key)
    }
}

impl<'a, E> Env for DenyListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        self
    }

    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        if !self.deny_list.contains(key) {
            self.env.as_mut().set(key, value);
        }
    }
}

impl<'a, E> AsMut<dyn Env + 'a> for DenyListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv>, Self: 'a {
    #[inline]
    fn as_mut(&mut self) -> &mut (dyn Env + 'a) {
        self
    }
}

impl<'a, E> AsRef<dyn GetEnv + 'a> for DenyListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv>, Self: 'a {
    #[inline]
    fn as_ref(&self) -> &(dyn GetEnv + 'a) {
        self
    }
}

pub struct AllowListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    env: E,
    allow_list: HashSet<&'a OsStr>,
}

impl<'a, E> AllowListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    #[inline]
    pub fn new(env: E, allow_list: HashSet<&'a OsStr>) -> Self {
        Self { env, allow_list }
    }

    #[inline]
    pub fn from_slice(env: E, allow_list: &'a [impl AsRef<OsStr>]) -> Self {
        Self {
            env,
            allow_list: allow_list.iter().map(|key| (*key).as_ref()).collect()
        }
    }

    #[inline]
    pub fn from_iter(env: E, allow_list: impl Iterator<Item=&'a OsStr>) -> Self {
        Self {
            env,
            allow_list: allow_list.collect()
        }
    }

    #[inline]
    pub fn env(&self) -> &E {
        &self.env
    }

    #[inline]
    pub fn allow_list(&self) -> &HashSet<&'a OsStr> {
        &self.allow_list
    }

    #[inline]
    pub fn into_env(self) -> E {
        self.env
    }
}

impl<'a, E> GetEnv for AllowListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    #[inline]
    fn get<'b>(&'b self, key: &OsStr) -> Option<Cow<'b, OsStr>> {
        self.env.as_ref().get(key)
    }
}

impl<'a, E> Env for AllowListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv> {
    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        self
    }

    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        if self.allow_list.contains(key) {
            self.env.as_mut().set(key, value);
        }
    }
}

impl<'a, E> AsMut<dyn Env + 'a> for AllowListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv>, Self: 'a {
    #[inline]
    fn as_mut(&mut self) -> &mut (dyn Env + 'a) {
        self
    }
}

impl<'a, E> AsRef<dyn GetEnv + 'a> for AllowListEnv<'a, E>
where E: AsMut<dyn Env> + AsRef<dyn GetEnv>, Self: 'a {
    #[inline]
    fn as_ref(&self) -> &(dyn GetEnv + 'a) {
        self
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
