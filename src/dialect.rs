use std::ffi::OsStr;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Dialect {
    Punktum,
    NodeJS,
    JavaScriptDotenv,
    PythonDotenvCLI,
    ComposeGo,
    Binary,
}

impl Default for Dialect {
    #[inline]
    fn default() -> Self {
        Dialect::Punktum
    }
}

impl std::fmt::Display for Dialect {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

impl TryFrom<&OsStr> for Dialect {
    type Error = IllegalDialect;

    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        if value.is_empty() ||
           value.eq_ignore_ascii_case("punktum") {
            Ok(Dialect::Punktum)
        } else if value.eq_ignore_ascii_case("nodejs") {
            Ok(Dialect::NodeJS)
        } else if value.eq_ignore_ascii_case("javascriptdotenv") ||
                  value.eq_ignore_ascii_case("jsdotenv") ||
                  value.eq_ignore_ascii_case("javascript-dotenv") ||
                  value.eq_ignore_ascii_case("js-dotenv") {
            Ok(Dialect::JavaScriptDotenv)
        } else if value.eq_ignore_ascii_case("pythondotenvcli") ||
                  value.eq_ignore_ascii_case("pydotenvcli") ||
                  value.eq_ignore_ascii_case("python-dotenv-cli") ||
                  value.eq_ignore_ascii_case("py-dotenv-cli") {
            Ok(Dialect::PythonDotenvCLI)
        } else if value.eq_ignore_ascii_case("composego") ||
                  value.eq_ignore_ascii_case("compose-go") {
            Ok(Dialect::ComposeGo)
        } else if value.eq_ignore_ascii_case("binary") {
            Ok(Dialect::Binary)
        } else {
            Err(IllegalDialect())
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct IllegalDialect();

impl std::fmt::Display for IllegalDialect {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "IllegalDialect".fmt(f)
    }
}

impl std::error::Error for IllegalDialect {}
