use std::{collections::HashMap, ffi::OsString};

use punktum::env::DenyListEnv;

const ENV: &[u8] = b"
PATH=/evil/binaries:$PATH
HOME=/root
PWD=/root
SHELL=/evil/shell
FOO=BAR
BAZ=BLA
";

fn main() -> punktum::Result<()> {
    let mut env = HashMap::<OsString, OsString>::new();
    punktum::build()
        .debug(true)
        .config_with_reader_and_parent(
            std::io::Cursor::new(ENV),
            &mut DenyListEnv::from_slice(&mut env, &["HOME", "PATH", "PWD", "SHELL"]),
            &punktum::env::EmptyEnv()
        )?;

    let mut env = env.iter().collect::<Vec<_>>();
    env.sort();
    for (key, value) in env {
        let key = key.to_string_lossy();
        println!("{key}={value:?}");
    }

    Ok(())
}
