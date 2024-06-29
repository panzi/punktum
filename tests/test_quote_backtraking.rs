mod quote_backtracking;
mod common;

use std::{collections::HashMap, ffi::{OsStr, OsString}};

use punktum::{self, build, Dialect, Result};

macro_rules! assert_quote_backtracking {
    ($fixture:expr, $dialect:expr $(, $override:ident)? $(, @parent: $parent:expr)?) => {
        let mut env = HashMap::<OsString, OsString>::new();

        assert_quote_backtracking!(@config
            build().
                strict(false).
                override_env(assert_quote_backtracking!(@override $($override)?)).
                dialect($dialect).
                path("tests/generate/quote-backtracking.env"),
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
fn test_quote_backtracking_java() -> Result<()> {
    assert_quote_backtracking!(quote_backtracking::java::FIXTURE, Dialect::JavaDotenv);
    Ok(())
}

#[test]
fn test_quote_backtracking_javascript() -> Result<()> {
    assert_quote_backtracking!(quote_backtracking::javascript::FIXTURE, Dialect::JavaScriptDotenv);
    Ok(())
}

#[test]
fn test_quote_backtracking_nodejs() -> Result<()> {
    assert_quote_backtracking!(quote_backtracking::nodejs::FIXTURE, Dialect::NodeJS);
    Ok(())
}

#[test]
fn test_quote_backtracking_punktum() -> Result<()> {
    assert_quote_backtracking!(quote_backtracking::punktum::FIXTURE, Dialect::Punktum);
    Ok(())
}

#[test]
fn test_quote_backtracking_python_cli() -> Result<()> {
    assert_quote_backtracking!(quote_backtracking::python_cli::FIXTURE, Dialect::PythonDotenvCLI);
    Ok(())
}

#[test]
fn test_quote_backtracking_python() -> Result<()> {
    assert_quote_backtracking!(quote_backtracking::python::FIXTURE, Dialect::PythonDotenv);
    Ok(())
}

#[test]
fn test_quote_backtracking_ruby() -> Result<()> {
    assert_quote_backtracking!(quote_backtracking::ruby::FIXTURE, Dialect::RubyDotenv);
    Ok(())
}
