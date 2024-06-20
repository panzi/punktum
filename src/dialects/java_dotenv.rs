// based on: https://github.com/cdimascio/dotenv-java/blob/0c5642eeac01cc3532d46e02d4901c58a9261961/src/main/java/io/github/cdimascio/dotenv/internal/DotenvParser.java
use std::{io::BufRead, path::Path};

use crate::{line_splitter::split_lines, Env, Error, Options, Result, DEBUG_PREFIX};

pub fn config_java_dotenv(reader: &mut dyn BufRead, env: &mut dyn Env, options: &Options<&Path>) -> Result<()> {
    let path_str = options.path.to_string_lossy();
    let mut lines = String::new();
    options.encoding.read_to_string(reader, &mut lines)?;

    let mut lineno = 0;
    for line in split_lines(&lines) {
        lineno += 1;

        let line = trim(line);
        // Don't know why it tests for an empty line twice in different ways?
        // That's what the original does!
        if is_empty_line(line) || is_comment(line) || is_blank(line) {
            continue;
        }

        // original regex: ^\s*([\w.\-]+)\s*(=)\s*('[^']*'|"[^"]*"|[^#]*)?\s*(#.*)?$

        let mut index = skip_ws(line, 0);
        let key_start = index;
        let key_end = find_vardef_end(line, index);
        index = key_end;

        if key_start == key_end {
            if options.debug {
                if let Some(ch) = line[key_start..].chars().next() {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{}: expected variable name, found {:?}: {}",
                        lineno, ch, line);
                } else {
                    eprintln!("{DEBUG_PREFIX}{path_str}:{}: unexpected end of file: {}",
                        lineno, line);
                }
            }
            if options.strict {
                return Err(Error::syntax_error(lineno, 1));
            }
            continue;
        }

        index = skip_ws(line, index);

        let Some(ch) = line[index..].chars().next() else {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}:{}: unexpected end of line: {}",
                    lineno, line);
            }
            if options.strict {
                return Err(Error::syntax_error(lineno, 1));
            }
            continue;
        };

        if ch != '=' {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}:{}: expected '=', found {:?}: {}",
                    lineno, ch, line);
            }
            if options.strict {
                return Err(Error::syntax_error(lineno, 1));
            }
            continue;
        }

        index = skip_ws(line, index + 1);

        let value_start = index;
        let mut value_end = index;

        if let Some(ch) = line[index..].chars().next() {
            if ch == '"' || ch == '\'' {
                if let Some(end_index) = line[value_start + 1..].find(ch) {
                    value_end = end_index + index + 2;
                }
            }
        }

        if value_start == value_end {
            // unquoted string
            value_end = if let Some(commet_start) = line[value_start..].find('#') {
                commet_start + value_start
            } else {
                line.len()
            };
        } else {
            // quoted string
            index = skip_ws(line, value_end);
            if let Some(ch) = line[index..].chars().next() {
                if ch != '#' {
                    if options.debug {
                        eprintln!("{DEBUG_PREFIX}{path_str}:{}: expected line end or '#', found {:?}, fallback to unquoted string: {}",
                            lineno, ch, line);
                    }
                    if options.strict {
                        return Err(Error::syntax_error(lineno, 1));
                    }
                    // fallback to unquoted string
                    value_end = if let Some(commet_start) = line[value_start..].find('#') {
                        commet_start + value_start
                    } else {
                        line.len()
                    };
                }
            }
        }

        let key = &line[key_start..key_end];
        let value = &line[value_start..value_end];
        if value == "\"" {
            if options.debug {
                eprintln!("{DEBUG_PREFIX}{path_str}:{}: value is a single double quote, this would have crashed the original: {}",
                    lineno, line);
            }
            if options.strict {
                return Err(Error::syntax_error(lineno, 1));
            }
            continue;
        }
        let value = normalize_value(value);
        options.set_var_cut_null(env, key, value);
    }

    Ok(())
}

#[inline]
fn skip_ws(src: &str, index: usize) -> usize {
    let Some(slice) = src.get(index..) else {
        return src.len();
    };
    let Some(pos) = slice.find(|ch| !is_space(ch)) else {
        return src.len();
    };
    pos + index
}

#[inline]
fn is_vardef(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-'
}

#[inline]
fn find_vardef_end(src: &str, index: usize) -> usize {
    let Some(slice) = src.get(index..) else {
        return src.len();
    };
    let Some(pos) = slice.find(|ch| !is_vardef(ch)) else {
        return src.len();
    };
    pos + index
}

// Java trim() compatible!
#[inline]
fn trim(src: &str) -> &str {
    src.trim_matches(|ch| ch <= ' ')
}

#[inline]
fn is_blank(src: &str) -> bool {
    trim(src).is_empty()
}

#[inline]
fn is_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r')
}

#[inline]
fn is_empty_line(src: &str) -> bool {
    !src.contains(|ch| !is_space(ch))
}

#[inline]
fn is_comment(src: &str) -> bool {
    src.starts_with('#') || src.starts_with("////")
}

#[inline]
fn is_quoted(src: &str) -> bool {
    src.starts_with('"') && src.ends_with('"')
}

#[inline]
fn normalize_value(value: &str) -> &str {
    let value = trim(value);
    if value.len() > 1 && is_quoted(value) {
        &value[1..value.len() - 1]
    } else {
        value
    }
}
