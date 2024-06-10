use std::{borrow::Cow, collections::HashMap, ffi::{OsStr, OsString}, path::Path};

use crate::{encoding::Encoding, env::{GetEnv, SystemEnv, SYSTEM_ENV}, Dialect, Env, Result, DEBUG_PREFIX};

#[derive(Debug, PartialEq, Clone)]
pub struct Options<P=&'static str>
where P: AsRef<Path> + Clone {
    /// Override existing environment variables.
    pub override_env: bool,

    /// Error on IO or parser errors.
    pub strict: bool,

    /// Log IO and parser errors.
    pub debug: bool,

    pub encoding: Encoding,

    pub dialect: Dialect,

    pub path: P,
}

pub const DEFAULT_PATH: &str = ".env";
pub const DEFAULT_OVERRIDE_ENV: bool = false;
pub const DEFAULT_STRICT: bool = true;
pub const DEFAULT_DEBUG: bool = false;

impl Default for Options {
    #[inline]
    fn default() -> Self {
        Self {
            override_env: DEFAULT_OVERRIDE_ENV,
            strict: DEFAULT_STRICT,
            debug: DEFAULT_DEBUG,
            encoding: Encoding::default(),
            dialect: Dialect::default(),
            path: DEFAULT_PATH,
        }
    }
}

impl<'a> Options<Cow<'a, OsStr>> {
    pub fn try_from(env: &'a impl GetEnv) -> Result<Self> {
        let override_env = env.get_override_env()?;
        let strict = env.get_strict()?;
        let debug = env.get_debug()?;
        let encoding = env.get_encoding()?;
        let dialect = env.get_dialect()?;
        let path = env.get_config_path();

        Ok(Self { override_env, strict, debug, encoding, dialect, path })
    }

    #[inline]
    pub fn try_from_env() -> Result<Self> {
        Self::try_from(&SYSTEM_ENV)
    }
}

impl<P> Options<P>
where P: AsRef<Path> + Clone {
    #[inline]
    pub fn with_path(path: P) -> Self {
        Self {
            override_env: DEFAULT_OVERRIDE_ENV,
            strict: DEFAULT_STRICT,
            debug: DEFAULT_DEBUG,
            encoding: Encoding::default(),
            dialect: Dialect::default(),
            path,
        }
    }

    #[inline]
    pub fn config(&self) -> Result<()> {
        crate::config_with_options(&mut SystemEnv(), &SYSTEM_ENV, self)
    }

    #[inline]
    pub fn config_env(&self, env: &mut impl Env) -> Result<()> {
        crate::config_with_options(env, &SYSTEM_ENV, self)
    }

    #[inline]
    pub fn config_with_parent(&self, env: &mut impl Env, parent: &impl GetEnv) -> Result<()> {
        crate::config_with_options(env, parent, self)
    }

    #[inline]
    pub fn config_new_with_parent(&self, parent: &impl GetEnv) -> Result<HashMap<OsString, OsString>> {
        let mut env = HashMap::new();
        crate::config_with_options(&mut env, parent, self)?;
        Ok(env)
    }

    #[inline]
    pub fn config_new(&self) -> Result<HashMap<OsString, OsString>> {
        let mut env = HashMap::new();
        crate::config_with_options(&mut env, &SYSTEM_ENV, self)?;
        Ok(env)
    }

    #[inline]
    pub(crate) fn set_var(&self, env: &mut dyn Env, key: &OsStr, value: &OsStr) {
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

    #[inline]
    pub(crate) fn set_var_cut_null(&self, env: &mut dyn Env, key: &str, value: &str) {
        let key = key.split('\0').next().unwrap();
        let value = value.split('\0').next().unwrap();
        self.set_var(env, key.as_ref(), value.as_ref());
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OptionType {
    Bool,
    Encoding,
    Dialect,
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
pub struct Builder<P=&'static str>
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
}

impl<'a> Builder<Cow<'a, OsStr>> {
    #[inline]
    pub fn try_from(env: &'a impl GetEnv) -> Result<Self> {
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
    pub fn with_path(path: P) -> Self {
        Self {
            options: Options::with_path(path)
        }
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
    pub fn dialect(mut self, value: Dialect) -> Self {
        self.options.dialect = value;
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
                dialect: self.options.dialect,
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
    pub fn config(self) -> Result<Self> {
        self.options.config()?;
        Ok(self)
    }

    #[inline]
    pub fn config_env(self, env: &mut impl Env) -> Result<Self> {
        self.options.config_env(env)?;
        Ok(self)
    }

    #[inline]
    pub fn config_with_parent(self, env: &mut impl Env, parent: &impl GetEnv) -> Result<Self> {
        self.options.config_with_parent(env, parent)?;
        Ok(self)
    }

    #[inline]
    pub fn config_new_with_parent(&self, parent: &impl GetEnv) -> Result<HashMap<OsString, OsString>> {
        self.options.config_new_with_parent(parent)
    }

    #[inline]
    pub fn config_new(&self) -> Result<HashMap<OsString, OsString>> {
        self.options.config_new()
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
