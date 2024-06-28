use std::{borrow::Cow, collections::HashSet, ffi::OsStr};

use super::{Env, GetEnv};

pub struct DenyListEnv<'a, E> {
    env: E,
    deny_list: HashSet<&'a OsStr>,
}

impl<'a, E> DenyListEnv<'a, E> {
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
where E: GetEnv {
    #[inline]
    fn get<'b>(&'b self, key: &OsStr) -> Option<Cow<'b, OsStr>> {
        self.env.get(key)
    }
}

impl<'a, E> Env for DenyListEnv<'a, E>
where E: Env {
    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        self
    }

    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        if !self.deny_list.contains(key) {
            self.env.set(key, value);
        }
    }

    #[inline]
    fn remove(&mut self, key: &OsStr) {
        if !self.deny_list.contains(key) {
            self.env.remove(key);
        }
    }
}

impl<'a, E> AsMut<DenyListEnv<'a, E>> for DenyListEnv<'a, E> {
    #[inline]
    fn as_mut(&mut self) -> &mut DenyListEnv<'a, E> {
        self
    }
}

impl<'a, E> AsRef<DenyListEnv<'a, E>> for DenyListEnv<'a, E> {
    #[inline]
    fn as_ref(&self) -> &DenyListEnv<'a, E> {
        self
    }
}
