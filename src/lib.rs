#![allow(
    clippy::manual_range_contains,
)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;

pub mod error;
use dialects::composego::config_composego;
use dialects::jsdotenv::config_jsdotenv;
use dialects::nodejs::config_nodejs;
use dialects::punktum::config_punktum;
use dialects::pydotenvcli::config_pydotenvcli;
use env::SYSTEM_ENV;
pub use error::Error;
pub use error::ErrorKind;

pub mod options;
pub use options::Options;
use options::Builder;

pub mod result;
pub use result::Result;

pub mod env;
pub use env::Env;
use env::{GetEnv, SystemEnv};

pub mod encoding;
pub use encoding::Encoding;

pub mod dialect;
pub use dialect::Dialect;

pub mod dialects;

pub(crate) const DEBUG_PREFIX: &str = concat!("[", env!("CARGO_PKG_NAME"), "@", env!("CARGO_PKG_VERSION"), "][DEBUG] ");

#[inline]
pub fn build() -> Builder {
    Builder::new()
}

#[inline]
pub fn build_from(env: &impl GetEnv) -> Result<Builder<Cow<'_, OsStr>>> {
    Builder::try_from(env)
}

#[inline]
pub fn build_from_env() -> Result<Builder<Cow<'static, OsStr>>> {
    Builder::try_from_env()
}

#[inline]
pub fn system_env() -> SystemEnv {
    SystemEnv::new()
}

#[inline]
pub fn config() -> Result<()> {
    let options = Options::try_from_env()?;
    config_with(&mut SystemEnv(), &SYSTEM_ENV, &options)
}

#[inline]
pub fn config_new() -> Result<HashMap<OsString, OsString>> {
    let options = Options::try_from_env()?;
    let mut env = HashMap::new();
    config_with(&mut env, &SYSTEM_ENV, &options)?;
    Ok(env)
}

#[inline]
pub fn config_with<P>(env: &mut impl Env, parent: &impl GetEnv, options: &Options<P>) -> Result<()>
where P: AsRef<Path> + Clone {
    let options = Options {
        override_env: options.override_env,
        strict: options.strict,
        debug: options.debug,
        encoding: options.encoding,
        dialect: options.dialect,
        path: options.path.as_ref(),
    };

    match options.dialect {
        Dialect::Punktum => config_punktum(env, parent, &options),
        Dialect::JavaScriptDotenv => config_jsdotenv(env, &options),
        Dialect::NodeJS => config_nodejs(env, &options),
        Dialect::PythonDotenvCLI => config_pydotenvcli(env, &options),
        Dialect::ComposeGo => config_composego(env, parent, &options),
    }
}

pub trait EnvWrite {
    fn write_env(&self, writer: impl std::io::Write) -> std::io::Result<()>;
}

pub fn write_var(mut writer: impl std::io::Write, key: impl AsRef<str>, value: impl AsRef<str>)  -> std::io::Result<()> {
    let key = key.as_ref();
    let mut value = value.as_ref();
    write!(writer, "{key}='")?;

    while let Some(index) = value.find('\'') {
        write!(writer, "{}'\"'\"'", &value[..index])?;
        value = &value[index + 1..];
    }

    writeln!(writer, "{value}'")?;

    Ok(())
}

pub fn write_iter(mut writer: impl std::io::Write, iter: impl Iterator<Item=(impl AsRef<str>, impl AsRef<str>)>) -> std::io::Result<()> {
    for (key, value) in iter {
        write_var(&mut writer, key, value)?;
    }
    Ok(())
}

impl<K, V> EnvWrite for HashMap<K, V>
where
    K: AsRef<str>,
    V: AsRef<str>
{
    #[inline]
    fn write_env(&self, writer: impl std::io::Write) -> std::io::Result<()> {
        write_iter(writer, self.iter())
    }
}
