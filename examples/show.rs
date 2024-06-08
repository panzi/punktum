use std::{collections::HashMap, ffi::OsString, path::{Path, PathBuf}};

fn main() -> punktum::Result<()> {
    let mut env = HashMap::<OsString, OsString>::new();
    env.insert(OsString::from("PRE_DEFINED"), OsString::from("no override"));
    let path =
        std::env::args_os().nth(1).map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("buggy-example.env"),
            PathBuf::from);

    punktum::build()
        .debug(true)
        .strict(false)
        .path(path)
        .config_env(&mut env)?;

    let mut env = env.iter().collect::<Vec<_>>();
    env.sort();
    for (key, value) in env {
        let key = key.to_string_lossy();
        println!("{key}={value:?}");
    }

    Ok(())
}
