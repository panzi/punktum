#!/usr/bin/env bash

set -eo pipefail

export DOTENV_CONFIG_STRICT=false
export DOTENV_CONFIG_ENCODING=utf8
export PRE_DEFINED="not override"

node --env-file=edge-cases.env gen_dotenv.js > edge_cases_nodejs.rs
DOTENV_CONFIG_PATH=edge-cases.env node gen_dotenv.js > edge_cases_javascript.rs

PYTHON_DOTENV_CLI=${PYTHON_DOTENV_CLI:-~/.local/bin/dotenv}
RUBY_DOTENV=${RUBY_DOTENV:-~/.rvm/gems/ruby-3.3.2/bin/dotenv}
GO_DOTENV=${GO_DOTENV:-godotenv}

dumpenv="$(cat dumpenv.js)"

set -x

# sed for patching their messed up encoding handling
"$PYTHON_DOTENV_CLI" --dotenv edge-cases.env node dumpenv.js | sed 's/Ã¤/ä/g' | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > edge_cases_python_cli.rs
"$PYTHON_DOTENV_CLI" --dotenv edge-cases.env --replace node -e "$dumpenv" | sed 's/Ã¤/ä/g' | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > edge_cases_python_cli_replace.rs

python -m dotenv --file edge-cases.env run --no-override node dumpenv.js | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > edge_cases_python.rs

"$RUBY_DOTENV" -f edge-cases.env node dumpenv.js > edge_cases_ruby.rs
DOTENV_LINEBREAK_MODE=legacy "$RUBY_DOTENV" -f edge-cases.env node dumpenv.js > edge_cases_ruby_legacy.rs

"$GO_DOTENV" -f godotenv.env node dumpenv.js > godotenv.rs

#dotenvy --file=dotenvy.env node dumpenv.js > dotenvy.rs

pushd java
gradle -q run --args="--file ../java.env node ../dumpenv.js" > ../java.rs
popd
