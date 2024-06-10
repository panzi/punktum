use std::{io::BufRead, path::Path};

use crate::{Env, Error, Options, Result, DEBUG_PREFIX};

pub fn config_binary(reader: &mut dyn BufRead, env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();

    let mut buf = Vec::new();
    let mut lineno = 0;
    loop {
        lineno += 1;
        buf.clear();
        let byte_count = reader.read_until(b'\0', &mut buf)?;

        if byte_count == 0 {
            break;
        }

        let line;

        if buf.ends_with(b"\0") {
            line = &buf[..buf.len() - 1];
        } else {
            let column = byte_count + 1;
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: line isn't terminated with a null byte");
            }

            if options.strict {
                return Err(Error::syntax_error(lineno, column));
            }
            line = &buf;
        }

        let Some(equals) = line.iter().cloned().position(|byte| byte == b'=') else {
            let column = byte_count + 1;
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: expected '='");
            }

            if options.strict {
                return Err(Error::syntax_error(lineno, column));
            }
            continue;
        };

        let key = &line[..equals];
        let value = &line[equals + 1..];

        let key = match String::from_utf8(key.to_vec()) {
            Ok(key) => key,
            Err(err) => {
                let column = 1;
                if options.debug {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: error decoding key: {err}");
                }

                if options.strict {
                    return Err(Error::syntax_error(lineno, column));
                }
                continue;
            }
        };

        let value = match String::from_utf8(value.to_vec()) {
            Ok(value) => value,
            Err(err) => {
                let column = equals + 2;
                if options.debug {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: error decoding value: {err}");
                }

                if options.strict {
                    return Err(Error::syntax_error(lineno, column));
                }
                continue;
            }
        };

        if key.is_empty() {
            let column = 1;
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}:{lineno}:{column}: empty keys are not allowed!");
            }

            if options.strict {
                return Err(Error::syntax_error(lineno, column));
            }
            continue;
        }

        options.set_var(env, key.as_ref(), value.as_ref());
    }

    Ok(())
}
