use std::{borrow::Cow, ffi::OsStr};

use super::GetEnv;


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
