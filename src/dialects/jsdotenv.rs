use std::{io::BufRead, path::Path};

use crate::{Env, Options, Result};

pub fn config_jsdotenv(_reader: &mut dyn BufRead, _env: &mut dyn Env, _options: &Options<&Path>) -> Result<()> {
    unimplemented!();
}
