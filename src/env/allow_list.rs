use std::{borrow::Cow, collections::HashSet, ffi::OsStr};

use super::{Env, GetEnv};

pub struct AllowListEnv<'a, E> {
    env: E,
    allow_list: HashSet<&'a OsStr>,
}

impl<'a, E> AllowListEnv<'a, E> {
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
where E: GetEnv {
    #[inline]
    fn get<'b>(&'b self, key: &OsStr) -> Option<Cow<'b, OsStr>> {
        self.env.get(key)
    }
}

impl<'a, E> Env for AllowListEnv<'a, E>
where E: Env {
    #[inline]
    fn as_get_env(&self) -> &dyn GetEnv {
        self
    }

    #[inline]
    fn set(&mut self, key: &OsStr, value: &OsStr) {
        if self.allow_list.contains(key) {
            self.env.set(key, value);
        }
    }

    #[inline]
    fn remove(&mut self, key: &OsStr) {
        if self.allow_list.contains(key) {
            self.env.remove(key);
        }
    }
}

impl<'a, E> AsMut<AllowListEnv<'a, E>> for AllowListEnv<'a, E> {
    #[inline]
    fn as_mut(&mut self) -> &mut AllowListEnv<'a, E> {
        self
    }
}

impl<'a, E> AsRef<AllowListEnv<'a, E>> for AllowListEnv<'a, E> {
    #[inline]
    fn as_ref(&self) -> &AllowListEnv<'a, E> {
        self
    }
}
