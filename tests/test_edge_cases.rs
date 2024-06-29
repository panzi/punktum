mod fixtures;

use std::{collections::HashMap, ffi::{OsStr, OsString}};

use punktum::{self, build, Dialect, Result};

const EDGE_CASES_PATH: &str = "tests/fixtures/edge-cases.env";

macro_rules! assert_env_eq {
    ($env:ident, $fixture:expr) => {
        for (key, expected_value) in $fixture {
            let actual_value = $env.get(OsStr::new(key));

            assert_eq!(actual_value.is_some(), true, "{key} is expected to be set, but isn't");
            let actual_value = actual_value.unwrap();
            assert_eq!(actual_value, expected_value, "{key} is expected to be {expected_value:?}, but is {actual_value:?}");
        }
    };
}

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
fn test_edge_cases_javascript() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_javascript::FIXTURE, Dialect::JavaScriptDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_nodejs() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_nodejs::FIXTURE, Dialect::NodeJS);
    Ok(())
}

#[test]
fn test_edge_cases_ruby() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_ruby::FIXTURE, Dialect::RubyDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_ruby_legacy() -> Result<()> {
    let mut parent = HashMap::new();
    parent.insert(OsString::from("DOTENV_LINEBREAK_MODE"), OsString::from("legacy"));
    assert_edge_cases!(fixtures::edge_cases_ruby_legacy::FIXTURE, Dialect::RubyDotenv, @parent: &parent);
    Ok(())
}

#[test]
fn test_edge_cases_python() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_python::FIXTURE, Dialect::PythonDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_python_cli() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_python_cli::FIXTURE, Dialect::PythonDotenvCLI, EDGE_CASES_PATH, override);
    Ok(())
}

#[test]
fn test_edge_cases_java() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_java::FIXTURE, Dialect::JavaDotenv, "tests/fixtures/java.env");
    Ok(())
}

#[test]
fn test_edge_cases_godotenv() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_godotenv::FIXTURE, Dialect::GoDotenv, "tests/fixtures/godotenv.env");
    Ok(())
}

#[test]
fn test_edge_cases_punktum() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_punktum::FIXTURE, Dialect::Punktum);
    Ok(())
}
