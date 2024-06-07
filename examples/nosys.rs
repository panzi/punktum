use std::path::Path;

use punktum::env::EmptyEnv;

fn main() -> punktum::Result<()> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("inherit.env");
    let env = punktum::build()
        .debug(true)
        .strict(false)
        .path(&path)
        .config_new()?;

    let mut env = env.iter().collect::<Vec<_>>();
    env.sort();
    println!("with system env:");
    for (key, value) in env {
        let key = key.to_string_lossy();
        println!("{key}={value:?}");
    }

    println!();

    let env = punktum::build()
        .debug(true)
        .strict(false)
        .path(&path)
        .config_new_with_parent(&EmptyEnv())?;

    let mut env = env.iter().collect::<Vec<_>>();
    env.sort();
    println!("without system env:");
    for (key, value) in env {
        let key = key.to_string_lossy();
        println!("{key}={value:?}");
    }

    Ok(())
}
