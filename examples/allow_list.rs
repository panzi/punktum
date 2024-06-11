use std::{collections::HashMap, ffi::OsString};

use punktum::env::AllowListEnv;

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
        .config_with_reader(
            std::io::Cursor::new(ENV),
            &mut AllowListEnv::from_slice(&mut env, &["FOO"]),
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
