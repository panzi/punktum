#![allow(
    clippy::manual_range_contains,
)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

pub mod error;
use dialects::binary::config_binary;
use dialects::composego::config_composego;
use dialects::go_dotenv::config_go_dotenv;
use dialects::java_dotenv::config_java_dotenv;
use dialects::javascript_dotenv::config_javascript_dotenv;
use dialects::nodejs::config_nodejs;
use dialects::punktum::config_punktum;
use dialects::python_dotenv::config_python_dotenv;
use dialects::python_dotenv_cli::config_python_dotenv_cli;
use dialects::ruby_dotenv::config_ruby_dotenv;
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

pub mod line_splitter;

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
    config_with_options(&mut SystemEnv(), &SYSTEM_ENV, &options)
}

#[inline]
pub fn config_new() -> Result<HashMap<OsString, OsString>> {
    let options = Options::try_from_env()?;
    let mut env = HashMap::new();
    config_with_options(&mut env, &SYSTEM_ENV, &options)?;
    Ok(env)
}

#[inline]
pub fn config_with_options<P>(env: &mut impl Env, parent: &impl GetEnv, options: &Options<P>) -> Result<()>
where P: AsRef<Path> {
    let path = options.path.as_ref();

    if path.as_os_str() == "-" {
        return config_with_reader(&mut std::io::stdin().lock(), env, parent, options);
    }

    let file = match File::open(path) {
        Err(err) => {
            if options.debug {
                let path_str = path.to_string_lossy();
                eprintln!("{DEBUG_PREFIX}{path_str}: {err}");
            }
            if options.strict {
                return Err(Error::with_cause(ErrorKind::IOError, err));
            }
            return Ok(());
        },
        Ok(file) => file,
    };
    let mut reader = BufReader::new(file);

    config_with_reader(&mut reader, env, parent, options)
}

#[inline]
pub fn config_with_reader<P>(reader: &mut dyn BufRead, env: &mut impl Env, parent: &impl GetEnv, options: &Options<P>) -> Result<()>
where P: AsRef<Path> {
    let options = Options {
        override_env: options.override_env,
        strict:       options.strict,
        debug:        options.debug,
        encoding:     options.encoding,
        dialect:      options.dialect,
        path:         options.path.as_ref(),
    };

    match options.dialect {
        Dialect::Punktum          => config_punktum(          reader, env, parent, &options),
        Dialect::JavaScriptDotenv => config_javascript_dotenv(reader, env, &options),
        Dialect::NodeJS           => config_nodejs(           reader, env, &options),
        Dialect::PythonDotenv     => config_python_dotenv(    reader, env, &options),
        Dialect::PythonDotenvCLI  => config_python_dotenv_cli(reader, env, &options),
        Dialect::ComposeGo        => config_composego(        reader, env, parent, &options),
        Dialect::GoDotenv         => config_go_dotenv(        reader, env, &options),
        Dialect::RubyDotenv       => config_ruby_dotenv(      reader, env, parent, &options),
        Dialect::JavaDotenv       => config_java_dotenv(      reader, env, &options),
        Dialect::Binary           => config_binary(           reader, env, &options),
    }
}

pub trait EnvWrite {
    fn write_env(&self, writer: impl std::io::Write) -> std::io::Result<()>;
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

pub fn write_var_binary(mut writer: impl std::io::Write, key: impl AsRef<str>, value: impl AsRef<str>)  -> std::io::Result<()> {
    let key = key.as_ref();
    let value = value.as_ref();
    write!(writer, "{key}={value}\0")?;

    Ok(())
}

pub fn write_iter_binary(mut writer: impl std::io::Write, iter: impl Iterator<Item=(impl AsRef<str>, impl AsRef<str>)>) -> std::io::Result<()> {
    for (key, value) in iter {
        write_var_binary(&mut writer, key, value)?;
    }
    Ok(())
}
