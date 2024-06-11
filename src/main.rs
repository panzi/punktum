use std::{collections::HashMap, env::ArgsOs, ffi::{OsStr, OsString}, io::Write, process::Command};

#[cfg(target_family = "unix")]
use std::os::unix::process::CommandExt;

use punktum::{env::parse_bool, options::{IllegalOption, OptionType}, Dialect, Encoding, Error, ErrorKind};

const USAGE: &str = concat!("\
usage: ", env!("CARGO_BIN_NAME"), " [--file=DOTENV...] [--replace] [--] command [args...]
       ", env!("CARGO_BIN_NAME"), " [--file=DOTENV...] [--replace] --print-env [--sorted] [--export] [--binary]
       ", env!("CARGO_BIN_NAME"), " [--help] [--version]

Punktum executes a given command with environment variables loaded from a .env file.

Positional arguments:
  command                   Program to execute.

Optional arguments:
  -h, --help                Print this help message and exit.
  -v, --version             Print program's version and exit.
  -f PATH, --file=PATH      File to use instead of \".env\"
                            This option can be passed multiple times.
                            All files are loaded in order.
                            Pass \"-\" to read from stdin.
  -r, --replace             Completely replace all existing environment variables with
                            the ones loaded from the .env file.
  -p, --print-env           Instead of running a command print the built environment
                            in a syntax compatible to Punktum and bash.
      --sorted              Sort printed environment variables for reproducible output.
      --export              Add 'export ' prefix to every printed environment variable.
      --strict=bool         Overwrite DOTENV_CONFIG_STRICT
      --debug=bool          Overwrite DOTENV_CONFIG_DEBUG
      --override=bool       Overwrite DOTENV_CONFIG_OVERRIDE
      --encoding=ENCODING   Overwrite DOTENV_CONFIG_ENCODING
      --dialect=DIALECT     Overwrite DOTENV_CONFIG_DIALECT

Environemnt variables:
  DOTENV_CONFIG_PATH=FILE  (default: .env)
    File to use instead of .env. This can be overwritten by --file.

  DOTENV_CONFIG_STRICT=true|false  (default: true)
    Stop and return an error if any problem is encounterd,
    like a file is not found, an encoding error, or a syntax error.

  DOTENV_CONFIG_DEBUG=true|false  (default: false)
    Write debug messages to stderr if there are any problems.

  DOTENV_CONFIG_OVERRIDE=true|false  (default: false)
    Replace existing environment variables.

  DOTENV_CONFIG_ENCODING=ENCODING
    Encoding of .env file.

    Supported values:
    - ASCII
    - ISO-8859-1  (alias: Latin1)
    - UTF-8       (default)
    - UTF-16BE
    - UTF-16LE
    - UTF-32BE
    - UTF-32LE

  DOTENV_CONFIG_DIALECT=DIALECT
    Dialect for the parser to use.

    Supported values:
    - Punktum (default)
    - NodeJS
    - PythonDotenvCLI
    - ComposeGo
    - Binary

© 2024 ", env!("CARGO_PKG_AUTHORS"), "
GitHub: https://github.com/panzi/punktum
");

fn parse_bool_option(option: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> punktum::Result<bool> {
    let value = value.as_ref();
    let Some(value) = parse_bool(value) else {
        return Err(Error::with_cause(
            ErrorKind::OptionsParseError,
            IllegalOption::new(
                option.as_ref().to_owned(),
                value.into(),
                OptionType::Bool)));
    };
    Ok(value)
}

fn parse_encoding_option(option: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> punktum::Result<Encoding> {
    let value = value.as_ref();
    let Ok(value) = Encoding::try_from(value) else {
        return Err(Error::with_cause(
            ErrorKind::OptionsParseError,
            IllegalOption::new(
                option.as_ref().to_owned(),
                value.into(),
                OptionType::Encoding)));
    };
    Ok(value)
}

fn parse_dialect_option(option: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> punktum::Result<Dialect> {
    let value = value.as_ref();
    let Ok(value) = Dialect::try_from(value) else {
        return Err(Error::with_cause(
            ErrorKind::OptionsParseError,
            IllegalOption::new(
                option.as_ref().to_owned(),
                value.into(),
                OptionType::Dialect)));
    };
    Ok(value)
}

fn require_arg(option: &OsStr, args: &mut ArgsOs) -> punktum::Result<OsString> {
    let Some(value) = args.next() else {
        let option = option.to_string_lossy();
        eprintln!("Error: {option} requires an argument");
        return Err(punktum::ErrorKind::IllegalArgument.into());
    };
    Ok(value)
}

fn exec() -> punktum::Result<()> {
    let mut args = std::env::args_os();
    let mut replace = false;
    let mut program = None;
    let mut files = vec![];
    let mut print_env: bool = false;
    let mut sorted: bool = false;
    let mut export: bool = false;
    let mut binary: bool = false;
    let mut debug: Option<bool> = None;
    let mut strict: Option<bool> = None;
    let mut override_env: Option<bool> = None;
    let mut encoding: Option<Encoding> = None;
    let mut dialect: Option<Dialect> = None;

    args.next();
    while let Some(arg) = args.next() {
        if arg == "--" {
            program = args.next();
            break;
        } else if arg == "-r" || arg == "--replace" {
            replace = true;
        } else if arg == "-p" || arg == "--print-env" {
            print_env = true;
        } else if arg == "--sorted" {
            sorted = true;
        } else if arg == "--export" {
            export = true;
        } else if arg == "--binary" {
            binary = true;
        } else if arg == "-f" || arg == "--file" {
            files.push(require_arg(&arg, &mut args)?);
        } else if arg == "--override" {
            let value = require_arg(&arg, &mut args)?;
            override_env = Some(parse_bool_option(&arg, &value)?);
        } else if arg == "--strict" {
            let value = require_arg(&arg, &mut args)?;
            strict = Some(parse_bool_option(&arg, &value)?);
        } else if arg == "--debug" {
            let value = require_arg(&arg, &mut args)?;
            debug = Some(parse_bool_option(&arg, &value)?);
        } else if arg == "--encoding" {
            let value = require_arg(&arg, &mut args)?;
            encoding = Some(parse_encoding_option(&arg, &value)?);
        } else if arg == "--dialect" {
            let value = require_arg(&arg, &mut args)?;
            dialect = Some(parse_dialect_option(&arg, &value)?);
        } else if arg == "-h" || arg == "--help" {
            print!("{USAGE}");
            return Ok(());
        } else if arg == "-v" || arg == "--version" {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        } else {
            let Some(str_arg) = arg.to_str() else {
                program = Some(arg);
                break;
            };

            if str_arg.starts_with("--file=") {
                files.push(OsStr::new(&str_arg[7..]).into());
            } else if str_arg.starts_with("--override=") {
                override_env = Some(parse_bool_option(&str_arg[..10], &str_arg[11..])?);
            } else if str_arg.starts_with("--strict=") {
                strict = Some(parse_bool_option(&str_arg[..8], &str_arg[9..])?);
            } else if str_arg.starts_with("--debug=") {
                debug = Some(parse_bool_option(&str_arg[..7], &str_arg[8..])?);
            } else if str_arg.starts_with("--encoding=") {
                encoding = Some(parse_encoding_option(&str_arg[..10], &str_arg[11..])?);
            } else if str_arg.starts_with("--dialect=") {
                dialect = Some(parse_dialect_option(&str_arg[..9], &str_arg[10..])?);
            } else if str_arg.starts_with('-') {
                eprintln!("Error: illegal argument: {arg:?}");
                return Err(punktum::ErrorKind::IllegalArgument.into());
            } else {
                program = Some(arg);
                break;
            }
        }
    }

    let mut env = if replace {
        HashMap::new()
    } else {
        punktum::system_env().to_hash_map()
    };

    let mut builder = punktum::build_from_env()?;

    if let Some(debug) = debug {
        builder = builder.debug(debug);
    }

    if let Some(strict) = strict {
        builder = builder.strict(strict);
    }

    if let Some(override_env) = override_env {
        builder = builder.override_env(override_env);
    }

    if let Some(encoding) = encoding {
        builder = builder.encoding(encoding);
    }

    if let Some(dialect) = dialect {
        builder = builder.dialect(dialect);
    }

    if files.is_empty() {
        builder.config_env(&mut env)?;
    } else {
        for file in files {
            builder.path(file).config_env(&mut env)?;
        }
    }

    if print_env {
        if binary && export {
            eprintln!("Error: Options --binary and --export are mutually exclusive!");
            return Err(punktum::ErrorKind::IllegalArgument.into());
        }

        if program.is_some() {
            eprintln!("Error: When --print-env is specified no command is expected!");
            return Err(punktum::ErrorKind::IllegalArgument.into());
        }

        let mut out = std::io::stdout().lock();

        if sorted {
            let mut sorted_env = vec![];

            for (key, value) in &env {
                let key = key.to_string_lossy();
                let value = value.to_string_lossy();
                sorted_env.push((key, value));
            }
            sorted_env.sort();

            if binary {
                punktum::write_iter_binary(&mut out, sorted_env.into_iter())?;
            } else {
                for (key, value) in sorted_env {
                    if export {
                        write!(out.by_ref(), "export ")?;
                    }
                    punktum::write_var(&mut out, key, value)?;
                }
            }
        } else {
            if binary {
                for (key, value) in env {
                    let key = key.to_string_lossy();
                    let value = value.to_string_lossy();
                    punktum::write_var_binary(&mut out, key, value)?;
                }
            } else {
                for (key, value) in env {
                    let key = key.to_string_lossy();
                    let value = value.to_string_lossy();
                    if export {
                        write!(out.by_ref(), "export ")?;
                    }
                    punktum::write_var(&mut out, key, value)?;
                }
            }
        }
        return Ok(());
    }

    if sorted {
        eprintln!("Error: Option --sorted is only to be used in combination with --print-env");
        return Err(punktum::ErrorKind::IllegalArgument.into());
    }

    if export {
        eprintln!("Error: Option --export is only to be used in combination with --print-env");
        return Err(punktum::ErrorKind::IllegalArgument.into());
    }

    if binary {
        eprintln!("Error: Option --binary is only to be used in combination with --print-env");
        return Err(punktum::ErrorKind::IllegalArgument.into());
    }

    let Some(program) = program else {
        return Err(punktum::ErrorKind::NotEnoughArguments.into())
    };

    let mut cmd = Command::new(program);
    let cmd = cmd.args(args).env_clear().envs(env);

    #[cfg(target_family = "unix")]
    return Err(punktum::Error::with_cause(
        punktum::ErrorKind::ExecError,
        cmd.exec()));

    #[cfg(not(target_family = "unix"))]
    {
        let status = cmd.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn main() {
    if let Err(error) = exec() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
