mod edge_cases;
mod common;

use std::{collections::HashMap, ffi::{OsStr, OsString}};

use punktum::{self, build, Dialect, Result};

const EDGE_CASES_PATH: &str = "tests/generate/edge-cases.env";

macro_rules! assert_edge_cases {
    ($fixture:expr, $dialect:expr $(, @parent: $parent:expr)?) => {
        assert_edge_cases!($fixture, $dialect, EDGE_CASES_PATH $(, @parent: $parent)?);
    };

    ($fixture:expr, $dialect:expr, $path:expr $(, $override:ident)? $(, @parent: $parent:expr)?) => {
        let mut env = HashMap::<OsString, OsString>::new();
        env.insert(OsString::from("PRE_DEFINED"), OsString::from("not override"));

        assert_edge_cases!(@config
            build().
                strict(false).
                override_env(assert_edge_cases!(@override $($override)?)).
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
fn test_edge_cases_godotenv() -> Result<()> {
    // godotenv fails loudly with a syntax error in some cases of edge-cases.env, so I use a more limited version.
    assert_edge_cases!(edge_cases::godotenv::FIXTURE, Dialect::GoDotenv, "tests/generate/edge-cases-godotenv.env");
    Ok(())
}

#[test]
fn test_edge_cases_java() -> Result<()> {
    // Java dotenv crashes (StringIndexOutOfBoundsException) in some cases of edge-cases.env, so I use a more limited version.
    assert_edge_cases!(edge_cases::java::FIXTURE, Dialect::JavaDotenv, "tests/generate/edge-cases-java.env");
    Ok(())
}

#[test]
fn test_edge_cases_javascript() -> Result<()> {
    assert_edge_cases!(edge_cases::javascript::FIXTURE, Dialect::JavaScriptDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_nodejs() -> Result<()> {
    assert_edge_cases!(edge_cases::nodejs::FIXTURE, Dialect::NodeJS);
    Ok(())
}

#[test]
fn test_edge_cases_punktum() -> Result<()> {
    assert_edge_cases!(edge_cases::punktum::FIXTURE, Dialect::Punktum);
    Ok(())
}

#[test]
fn test_edge_cases_python_cli() -> Result<()> {
    assert_edge_cases!(edge_cases::python_cli::FIXTURE, Dialect::PythonDotenvCLI, EDGE_CASES_PATH, override);
    Ok(())
}

#[test]
fn test_edge_cases_python() -> Result<()> {
    assert_edge_cases!(edge_cases::python::FIXTURE, Dialect::PythonDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_ruby_legacy() -> Result<()> {
    let mut parent = HashMap::new();
    parent.insert(OsString::from("DOTENV_LINEBREAK_MODE"), OsString::from("legacy"));
    assert_edge_cases!(edge_cases::ruby_legacy::FIXTURE, Dialect::RubyDotenv, @parent: &parent);
    Ok(())
}

#[test]
fn test_edge_cases_ruby() -> Result<()> {
    assert_edge_cases!(edge_cases::ruby::FIXTURE, Dialect::RubyDotenv);
    Ok(())
}
