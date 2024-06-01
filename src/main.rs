use std::process::Command;

#[cfg(target_family = "unix")]
use std::os::unix::process::CommandExt;

fn run() -> dotenv::Result<()> {
    dotenv::load()?;

    let mut args = std::env::args_os();
    if let Some(program) = args.nth(1) {
        #[cfg(target_family = "unix")]
        return Err(dotenv::Error::new(
            dotenv::ErrorKind::ExecError,
            Command::new(program).args(args).exec()));

        #[cfg(not(target_family = "unix"))]
        {
            let status = Command::new(program).args(args).status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
    } else {
        return Err(dotenv::ErrorKind::NotEnoughArguments.into());
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
