#!/usr/bin/env bash

set -xeo pipefail

SELF=$(readlink -f "$0")
DIR=$(dirname "$SELF")

cd "$DIR"

export DOTENV_CONFIG_STRICT=false
export DOTENV_CONFIG_ENCODING=utf8
export PRE_DEFINED="not override"

PYTHON_DOTENV_CLI=${PYTHON_DOTENV_CLI:-~/.local/bin/dotenv}
RUBY_DOTENV=${RUBY_DOTENV:-~/.rvm/gems/ruby-3.3.2/bin/dotenv}
GO_DOTENV=${GO_DOTENV:-godotenv}

# edge-cases.env
# ==============
node --env-file=edge-cases.env gen_dotenv.js > ../edge_cases/nodejs.rs
DOTENV_CONFIG_PATH=edge-cases.env node gen_dotenv.js > ../edge_cases/javascript.rs

# sed for patching their messed up encoding handling
"$PYTHON_DOTENV_CLI" --dotenv edge-cases.env node dumpenv.js | sed 's/Ã¤/ä/g' | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > ../edge_cases/python_cli.rs

python -m dotenv --file edge-cases.env run --no-override node dumpenv.js | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > ../edge_cases/python.rs

"$RUBY_DOTENV" -f edge-cases.env node dumpenv.js > ../edge_cases/ruby.rs
DOTENV_LINEBREAK_MODE=legacy "$RUBY_DOTENV" -f edge-cases.env node dumpenv.js > ../edge_cases/ruby_legacy.rs

"$GO_DOTENV" -f godotenv.env node dumpenv.js > ../edge_cases/godotenv.rs

#dotenvy --file=dotenvy.env node dumpenv.js > ../edge_cases/dotenvy.rs

pushd java
gradle -q run --args="--file ../java.env node ../dumpenv.js" > ../../edge_cases/java.rs
popd

pushd ../..
cargo run -- --strict=false --file=tests/generate/edge-cases.env node tests/generate/dumpenv.js | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > tests/edge_cases/punktum.rs
popd

# quote-backtracking.env
# ======================

keys=(FOO world BAR)
node --env-file=quote-backtracking.env gen_dotenv.js "${keys[@]}" > ../quote_backtracking/nodejs.rs
DOTENV_CONFIG_PATH=quote-backtracking.env node gen_dotenv.js "${keys[@]}" > ../quote_backtracking/javascript.rs

# sed for patching their messed up encoding handling
"$PYTHON_DOTENV_CLI" --dotenv quote-backtracking.env node dumpenv.js "${keys[@]}" | sed 's/Ã¤/ä/g' | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > ../quote_backtracking/python_cli.rs

python -m dotenv --file quote-backtracking.env run --no-override node dumpenv.js "${keys[@]}" | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > ../quote_backtracking/python.rs

"$RUBY_DOTENV" -f quote-backtracking.env node dumpenv.js "${keys[@]}" > ../quote_backtracking/ruby.rs

# godotenv just gives an error for that

pushd java
gradle -q run --args="--file ../quote-backtracking.env node ../dumpenv.js ${keys[*]}" > ../../quote_backtracking/java.rs
popd

pushd ../..
cargo run -- --strict=false --file=tests/generate/quote-backtracking.env node tests/generate/dumpenv.js "${keys[@]}" | sed 's/:\\b/:\\u{8}/g' | sed 's/:\\f/:\\u{C}/g' > tests/quote_backtracking/punktum.rs
popd
