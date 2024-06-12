use std::{borrow::Cow, collections::HashMap, ffi::{OsStr, OsString}, sync::Mutex};

use super::{Env, GetEnv};

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
