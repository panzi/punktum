use std::str::CharIndices;
use std::path::Path;

pub mod error;
use dialects::nodejs::config_nodejs;
use dialects::punktum::config_punktum;
use dialects::pydotenvcli::config_pydotenvcli;
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
pub fn build_from<E: GetEnv>(env: &E) -> Result<Builder> {
    Builder::try_from(env)
}

#[inline]
pub fn build_from_env() -> Result<Builder> {
    Builder::try_from_env()
}

#[inline]
pub fn system_env() -> SystemEnv {
    SystemEnv::get()
}

#[inline]
pub fn config() -> Result<()> {
    let options = Options::try_from_env()?;
    config_with(&mut SystemEnv::get(), &options)
}

#[inline]
pub fn config_with<P, E: Env>(env: &mut E, options: &Options<P>) -> Result<()>
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
        Dialect::JavaScriptDotenv => unimplemented!(),
        Dialect::NodeJS => config_nodejs(env, &options),
        Dialect::PythonDotenvCLI => config_pydotenvcli(env, &options),
        Dialect::Punktum => config_punktum(env, &options)
    }
}

pub(crate) fn skipws(iter: &mut CharIndices) -> Option<(usize, char)> {
    while let Some((index, ch)) = iter.next() {
        if !ch.is_ascii_whitespace() {
            return Some((index, ch));
        }
    }

    None
}

pub(crate) fn skip_word(iter: &mut CharIndices) -> Option<(usize, char)> {
    while let Some((index, ch)) = iter.next() {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return Some((index, ch));
        }
    }

    None
}
