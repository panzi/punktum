use std::{collections::HashMap, process::Command};

#[cfg(target_family = "unix")]
use std::os::unix::process::CommandExt;

fn exec() -> punktum::Result<()> {
    let mut args = std::env::args_os();
    let mut replace = false;
    let mut program = None;

    args.next();
    while let Some(arg) = args.next() {
        if arg == "--" {
            program = args.next();
            break;
        } else if arg == "-r" || arg == "--replace" {
            replace = true;
        } else if arg.to_str().map_or(false, |arg| arg.starts_with('-')) {
            eprintln!("Error: illegal argument: {arg:?}");
            return Err(punktum::ErrorKind::IllegalArgument.into());
        } else {
            program = Some(arg);
            break;
        }
    }

    if let Some(program) = program {
        let mut env = if replace {
            HashMap::new()
        } else {
            punktum::system_env().to_hash_map()
        };

        punktum::build_from_env()?.
            config_env(&mut env)?;

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
