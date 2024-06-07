use std::{collections::HashMap, ffi::OsString, path::Path};

fn main() -> punktum::Result<()> {
    let mut env = HashMap::<OsString, OsString>::new();
    env.insert(OsString::from("PRE_DEFINED"), OsString::from("no override"));

    punktum::build()
        .debug(true)
        .strict(false)
        .path(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("buggy-example.env"))
        .config_env(&mut env)?;

    let mut env = env.iter().collect::<Vec<_>>();
    env.sort();
    for (key, value) in env {
        let key = key.to_string_lossy();
        println!("{key}={value:?}");
    }

    Ok(())
}
