mod varsubst;
mod common;

use std::io::Cursor;

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
                path("tests/generate/varsubst.env"),
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
fn test_composego_invalid1() {
    let res = build().
        dialect(Dialect::ComposeGo).
        config_new_with_reader(Cursor::new(b"
FOO=\"${BAR:-${BAZ}
}\"
"));
    assert_eq!(res.is_err(), true);
}

#[test]
fn test_composego_invalid2() {
    let res = build().
        dialect(Dialect::ComposeGo).
        config_new_with_reader(Cursor::new(b"
FOO=\"${BAR:-
}\"
"));
    assert_eq!(res.is_err(), true);
}

#[test]
fn test_composego_invalid3() {
    let res = build().
        dialect(Dialect::ComposeGo).
        config_new_with_reader(Cursor::new(b"
EMPTY=
FOO=\"${EMPTY:?message}\"
"));
    assert_eq!(res.is_err(), true);
}

#[test]
fn test_composego_invalid4() {
    let res = build().
        dialect(Dialect::ComposeGo).
        config_new_with_reader(Cursor::new(b"
FOO=\"${BAR?message}\"
"));
    assert_eq!(res.is_err(), true);
}

#[test]
fn test_varsubst_composego() -> Result<()> {
    assert_varsubst!(varsubst::composego::FIXTURE, Dialect::ComposeGo);
    Ok(())
}

#[test]
fn test_punktum_invalid1() {
    let res = build().
        dialect(Dialect::Punktum).
        config_new_with_reader(Cursor::new(b"
EMPTY=
FOO=\"${EMPTY:?message}\"
"));
    assert_eq!(res.is_err(), true);
}

#[test]
fn test_punktum_invalid2() {
    let res = build().
        dialect(Dialect::Punktum).
        config_new_with_reader(Cursor::new(b"
FOO=\"${BAR?message}\"
"));
    assert_eq!(res.is_err(), true);
}

#[test]
fn test_varsubst_punktum() -> Result<()> {
    assert_varsubst!(varsubst::punktum::FIXTURE, Dialect::Punktum);
    Ok(())
}

#[test]
fn test_varsubst_python() -> Result<()> {
    assert_varsubst!(varsubst::python::FIXTURE, Dialect::PythonDotenv);
    Ok(())
}

#[test]
fn test_varsubst_ruby() -> Result<()> {
    assert_varsubst!(varsubst::ruby::FIXTURE, Dialect::RubyDotenv);
    Ok(())
}
