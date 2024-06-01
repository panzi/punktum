use std::process::Command;

#[cfg(target_family = "unix")]
use std::os::unix::process::CommandExt;

fn main() -> dotenv::Result<()> {
    dotenv::load()?;

    let mut args = std::env::args_os();
    if let Some(program) = args.next() {
        #[cfg(target_family = "unix")]
        return Err(dotenv::Error::with_cause(
            dotenv::ErrorKind::ExecError,
            Box::new(Command::new(program).args(args).exec())));

        #[cfg(not(target_family = "unix"))]
        {
            let status = Command::new(program).args(args).status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}
