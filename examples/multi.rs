use std::{collections::HashMap, path::Path};

fn main() -> punktum::Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut env = HashMap::<String, String>::new();
    punktum::build()
        .debug(true)
        .strict(false)
        .path(dir.join("env1.env")).config_env(&mut env)?
        .path(dir.join("env2.env")).config_env(&mut env)?
        .override_env(true)
        .path(dir.join("env3.env")).config_env(&mut env)?;

    let mut env = env.iter().collect::<Vec<_>>();
    env.sort();
    for (key, value) in env {
        println!("{key}={value:?}");
    }

    Ok(())
}
