use std::{fs::File, io::BufReader, path::Path};

use crate::{Env, Error, ErrorKind, Options, Result, DEBUG_PREFIX};

// Trying to emulate: https://github.com/nodejs/node/blob/v22.x/src/node_dotenv.cc
// FIXME: doesn't work!
pub fn config_nodejs(env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();

    let mut lines = match File::open(options.path) {
        Err(err) => {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}: {err}");
            }
            if options.strict {
                return Err(Error::with_cause(ErrorKind::IOError, err));
            }
            return Ok(());
        }
        Ok(file) => {
            let mut lines = String::new();
            options.encoding.read_to_string(&mut BufReader::new(file), &mut lines)?;
            lines
        }
    };

    lines.retain(|ch| ch != '\r');
    let mut content = lines.trim_matches(' ');

    while !content.is_empty() {
        // skip empty lines and comments
        if content.starts_with('\n') || content.starts_with('#') {
            if let Some(newline) = content.find('\n') {
                content = &content[newline + 1..];
                continue;
            }
        }

        let Some(equal) = content.find('=') else {
            break;
        };

        let mut key = &content[..equal];
        content = &content[equal + 1..];
        key = key.trim_matches(' ');

        if key.is_empty() {
            break;
        }

        // remove export prefix from key
        if key.starts_with("export ") {
            key = &key[7..];
        }
        // eprintln!(">>> key: {key:?}");
        // eprintln!(">>> rest: {:?}...", &content[..6.min(content.len())]);

        // SAFETY: Content is guaranteed to have at least one character
        if content.is_empty() {
            // In case the last line is a single key without value
            // Example: KEY= (without a newline at the EOF)
            options.set_var(env, key.split('\0').next().unwrap().as_ref(), "".as_ref());
            break;
        }

        // Expand new line if \n it's inside double quotes
        // Example: EXPAND_NEWLINES = 'expand\nnew\nlines'
        if content.starts_with('"') {
            if let Some(closing_quote) = content[1..].find('"') {
                let value = &content[1..closing_quote + 1];
                let multi_line_value = value.replace("\\n", "\n");
                options.set_var(env,
                    key.split('\0').next().unwrap().as_ref(),
                    multi_line_value.split('\0').next().unwrap().as_ref());
                content = &content[closing_quote + 1..];
                let newline = content.find('\n').unwrap_or(content.len());
                content = &content[newline..];
                continue;
            }
        }

        // Check if the value is wrapped in quotes, single quotes or backticks
        let front = content.chars().next().unwrap();
        if front == '\'' || front == '"' || front == '`' {
            // Check if the closing quote is not found
            // Example: KEY="value
            if let Some(closing_quote) = content[1..].find(front) {
                // Example: KEY="value"
                let value = &content[1..closing_quote + 1];
                options.set_var(env,
                    key.split('\0').next().unwrap().as_ref(),
                    value.split('\0').next().unwrap().as_ref());
                // Select the first newline after the closing quotation mark
                // since there could be newline characters inside the value.
                content = &content[closing_quote + 1..];
                let newline = content.find('\n').unwrap_or(content.len());
                content = &content[newline..];
            } else {
                // Check if newline exist. If it does, take the entire line as the value
                // Example: KEY="value\nKEY2=value2
                // The value pair should be `"value`
                if let Some(newline) = content.find('\n') {
                    let value = &content[..newline];
                    options.set_var(env,
                        key.split('\0').next().unwrap().as_ref(),
                        value.split('\0').next().unwrap().as_ref());
                    content = &content[newline..];
                }
            }
        } else {
            // Regular key value pair.
            // Example: `KEY=this is value`
            let mut value;
            if let Some(newline) = content.find('\n') {
                value = &content[..newline];
                //eprintln!(">>> {key}={:?} <<<", value);
                // Check if there is a comment in the line
                // Example: KEY=value # comment
                // The value pair should be `value`
                if let Some(hash_character) = value.find('#') {
                    value = &value[..hash_character];
                }
                content = &content[newline..];
            } else {
                // In case the last line is a single key/value pair
                // Example: KEY=VALUE (without a newline at the EOF)
                value = content;
            }

            value = value.trim_matches(' ');
            options.set_var(env,
                key.split('\0').next().unwrap().as_ref(),
                value.split('\0').next().unwrap().as_ref());
        }
    }

    Ok(())
}
