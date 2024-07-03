mod varsubst_ext;
mod common;

use std::{collections::HashMap, ffi::{OsStr, OsString}};

use punktum::{self, build, Dialect, Result};

macro_rules! assert_varsubst {
    ($fixture:expr, $dialect:expr $(, $override:ident)? $(, @parent: $parent:expr)?) => {
        let mut env = HashMap::<OsString, OsString>::new();

        assert_varsubst!(@config
            build().
                strict(false).
                override_env(assert_varsubst!(@override $($override)?)).
                dialect($dialect).
                path("tests/generate/varsubst-ext.env"),
            &mut env
            $(, $parent)?)?;

        assert_env_eq!(env, $fixture);
    };

    (@config $builder:expr, $env:expr, $parent:expr) => {
        $builder.config_with_parent($env, $parent)
    };

    (@config $builder:expr, $env:expr) => {
        $builder.config_env($env)
    };

    (@override) => { false };

    (@override override) => { true };
}

#[test]
fn test_varsubst_ext_composego() -> Result<()> {
    assert_varsubst!(varsubst_ext::composego::FIXTURE, Dialect::ComposeGo);
    Ok(())
}

#[test]
fn test_varsubst_ext_punktum() -> Result<()> {
    assert_varsubst!(varsubst_ext::punktum::FIXTURE, Dialect::Punktum);
    Ok(())
}

#[test]
fn test_varsubst_ext_python() -> Result<()> {
    assert_varsubst!(varsubst_ext::python::FIXTURE, Dialect::PythonDotenv);
    Ok(())
}
