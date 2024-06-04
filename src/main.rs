use std::process::Command;

#[cfg(target_family = "unix")]
use std::os::unix::process::CommandExt;

fn exec() -> punktum::Result<()> {
    punktum::config()?;

    let mut args = std::env::args_os();
    if let Some(program) = args.nth(1) {
        #[cfg(target_family = "unix")]
        return Err(punktum::Error::with_cause(
            punktum::ErrorKind::ExecError,
            Command::new(program).args(args).exec()));

        #[cfg(not(target_family = "unix"))]
        {
            let status = Command::new(program).args(args).status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
    } else {
        return Err(punktum::ErrorKind::NotEnoughArguments.into());
    }
}

fn main() {
    if let Err(error) = exec() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
