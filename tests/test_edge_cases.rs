mod fixtures;

use std::{collections::HashMap, ffi::{OsStr, OsString}};

use punktum::{self, build, Dialect, Result};

const EDGE_CASES_PATH: &str = "tests/fixtures/edge-cases.env";

macro_rules! assert_edge_cases {
    ($fixture:expr, $dialect:expr) => {
        assert_edge_cases!($fixture, $dialect, EDGE_CASES_PATH);
    };

    ($fixture:expr, $dialect:expr, $path:expr) => {
        let mut env = HashMap::<OsString, OsString>::new();
        env.insert(OsString::from("PRE_DEFINED"), OsString::from("not override"));

        build().
            strict(false).
            override_env(false).
            dialect($dialect).
            path(EDGE_CASES_PATH).
            config_env(&mut env)?;

        for (key, expected_value) in $fixture {
            let actual_value = env.get(OsStr::new(key));

            assert_eq!(actual_value.is_some(), true, "{key} is expected to be set, but isn't");
            let actual_value = actual_value.unwrap();
            assert_eq!(actual_value, expected_value, "{key} is expected to be {expected_value:?}, but is {actual_value:?}");
        }
    };
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
fn test_edge_cases_python() -> Result<()> {
    assert_edge_cases!(fixtures::edge_cases_python::FIXTURE, Dialect::PythonDotenv);
    Ok(())
}

#[test]
fn test_edge_cases_java() -> Result<()> {
    assert_edge_cases!(fixtures::java::FIXTURE, Dialect::JavaDotenv, "tests/fixtures/java.env");
    Ok(())
}

/*
#[test]
fn test_edge_cases_godotenv() -> Result<()> {
    assert_edge_cases!(fixtures::godotenv::FIXTURE, Dialect::JavaDotenv, "tests/fixtures/godotenv.env");
    Ok(())
}
*/
