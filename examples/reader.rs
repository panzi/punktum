const ENV: &[u8] = b"
FOO=BAR
MESSAGE=\"This is in memory!\\nFOO=${FOO}\"
";

fn main() -> punktum::Result<()> {
    let env = punktum::build()
        .debug(true)
        .strict(false)
        .config_new_with_reader_and_parent(
            std::io::Cursor::new(ENV),
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
