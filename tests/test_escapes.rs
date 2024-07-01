mod common;
mod escapes;

use std::{collections::HashMap, ffi::{OsStr, OsString}};

use punktum::{self, build, Dialect, Result};

const ESCAPES_PATH: &str = "tests/generate/escapes.env";

macro_rules! assert_escapes {
    ($fixture:expr, $dialect:expr $(, @parent: $parent:expr)?) => {
        assert_escapes!($fixture, $dialect, ESCAPES_PATH $(, @parent: $parent)?);
    };

    ($fixture:expr, $dialect:expr, $path:expr $(, $override:ident)? $(, @parent: $parent:expr)?) => {
        let mut env = HashMap::<OsString, OsString>::new();

        assert_escapes!(@config
            build().
                strict(false).
                override_env(assert_escapes!(@override $($override)?)).
                dialect($dialect).
                path($path),
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
fn test_edge_cases_composego() -> Result<()> {
    assert_escapes!(escapes::composego::FIXTURE, Dialect::ComposeGo);
    Ok(())
}

#[test]
fn test_edge_cases_godotenv() -> Result<()> {
    // godotenv fails loudly with a syntax error in some cases of edge-cases.env, so I use a more limited version.
    assert_escapes!(escapes::godotenv::FIXTURE, Dialect::GoDotenv, "tests/generate/escapes-godotenv.env");
    Ok(())
}

#[test]
fn test_edge_cases_java() -> Result<()> {
    assert_escapes!(escapes::java::FIXTURE, Dialect::JavaDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_javascript() -> Result<()> {
    assert_escapes!(escapes::javascript::FIXTURE, Dialect::JavaScriptDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_nodejs() -> Result<()> {
    assert_escapes!(escapes::nodejs::FIXTURE, Dialect::NodeJS);
    Ok(())
}

#[test]
fn test_edge_cases_punktum() -> Result<()> {
    assert_escapes!(escapes::punktum::FIXTURE, Dialect::Punktum);
    Ok(())
}

#[test]
fn test_edge_cases_python_cli() -> Result<()> {
    assert_escapes!(escapes::python_cli::FIXTURE, Dialect::PythonDotenvCLI, "tests/generate/escapes-python-cli.env", override);
    Ok(())
}

#[test]
fn test_edge_cases_python() -> Result<()> {
    assert_escapes!(escapes::python::FIXTURE, Dialect::PythonDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_ruby_legacy() -> Result<()> {
    let mut parent = HashMap::new();
    parent.insert(OsString::from("DOTENV_LINEBREAK_MODE"), OsString::from("legacy"));
    assert_escapes!(escapes::ruby_legacy::FIXTURE, Dialect::RubyDotenv, @parent: &parent);
    Ok(())
}

#[test]
fn test_edge_cases_ruby() -> Result<()> {
    assert_escapes!(escapes::ruby::FIXTURE, Dialect::RubyDotenv);
    Ok(())
}
