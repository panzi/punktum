use std::{collections::HashMap, ffi::OsStr, process::Command};

#[cfg(target_family = "unix")]
use std::os::unix::process::CommandExt;

const USAGE: &str = "\
usage: punktum [--help] [--file=DOTENV] [--version] [--replace] [--] command [args...]

Punktum executes a given command with environment variables loaded from a .env file.

Positional arguments:
  command                   Program to execute.

Optional arguments:
  -h, --help                Print this help message and exit.
  -v, --version             Print program's version and exit.
  -f DOTENV, --file=DOTENV  File to use instead of .env
                            This option can be passed multiple times. All files are loaded in order.
  -r, --replace             Completely replace all existing environment variables with the ones loaded from the .env file.

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
    - ISO-8859-1 (alias: Latin1)
    - UTF-8 (default)
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
    - GoDotenv
    - JavaScriptDotenv
";

fn exec() -> punktum::Result<()> {
    let mut args = std::env::args_os();
    let mut replace = false;
    let mut program = None;
    let mut files = vec![];

    args.next();
    while let Some(arg) = args.next() {
        if arg == "--" {
            program = args.next();
            break;
        } else if arg == "-r" || arg == "--replace" {
            replace = true;
        } else if arg == "-f" || arg == "--file" {
            let Some(file) = args.next() else {
                let arg = arg.to_string_lossy();
                eprintln!("Error: {arg} requires an argument");
                return Err(punktum::ErrorKind::IllegalArgument.into());
            };
            files.push(file);
        } else if arg == "-h" || arg == "--help" {
            println!("{USAGE}");
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
            } else if str_arg.starts_with('-') {
                eprintln!("Error: illegal argument: {arg:?}");
                return Err(punktum::ErrorKind::IllegalArgument.into());
            } else {
                program = Some(arg);
                break;
            }
        }
    }

    if let Some(program) = program {
        let mut env = if replace {
            HashMap::new()
        } else {
            punktum::system_env().to_hash_map()
        };

        let builder = punktum::build_from_env()?;

        if files.is_empty() {
            builder.config_env(&mut env)?;
        } else {
            for file in files {
                builder.path(file).config_env(&mut env)?;
            }
        }

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
    } else {
        Err(punktum::ErrorKind::NotEnoughArguments.into())
    }
}

fn main() {
    if let Err(error) = exec() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
