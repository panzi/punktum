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

pushd compose-go
go build -o dotenv
popd

# edge-cases.env
# ==============

node --env-file=edge-cases.env gen_dotenv.js > ../edge_cases/nodejs.rs
DOTENV_CONFIG_PATH=edge-cases.env node gen_dotenv.js > ../edge_cases/javascript.rs

"$PYTHON_DOTENV_CLI" --dotenv edge-cases.env node dumpenv.js > ../edge_cases/python_cli.rs

python -m dotenv --file edge-cases.env run --no-override node dumpenv.js > ../edge_cases/python.rs

"$RUBY_DOTENV" -f edge-cases.env node dumpenv.js > ../edge_cases/ruby.rs
DOTENV_LINEBREAK_MODE=legacy "$RUBY_DOTENV" -f edge-cases.env node dumpenv.js > ../edge_cases/ruby_legacy.rs

"$GO_DOTENV" -f edge-cases-godotenv.env node dumpenv.js > ../edge_cases/godotenv.rs
#INHERIT=inherited compose-go/dotenv --file edge-cases.env node dumpyenv.js > ../edge_cases/composego.rs
# TODO: compose-go dotenv (supports INHERIT=inherited )

#dotenvy --file=dotenvy.env node dumpenv.js > ../edge_cases/dotenvy.rs

pushd java
gradle -q run --args="--file ../edge-cases-java.env node ../dumpenv.js" > ../../edge_cases/java.rs
popd

pushd ../..
INHERIT=inherited cargo run -- --strict=false --file=tests/generate/edge-cases.env node tests/generate/dumpenv.js > tests/edge_cases/punktum.rs
popd

# quote-backtracking.env
# ======================

keys=(FOO world BAR)
node --env-file=quote-backtracking.env gen_dotenv.js "${keys[@]}" > ../quote_backtracking/nodejs.rs
DOTENV_CONFIG_PATH=quote-backtracking.env node gen_dotenv.js "${keys[@]}" > ../quote_backtracking/javascript.rs

"$PYTHON_DOTENV_CLI" --dotenv quote-backtracking.env node dumpenv.js "${keys[@]}" > ../quote_backtracking/python_cli.rs

python -m dotenv --file quote-backtracking.env run --no-override node dumpenv.js "${keys[@]}" > ../quote_backtracking/python.rs

"$RUBY_DOTENV" -f quote-backtracking.env node dumpenv.js "${keys[@]}" > ../quote_backtracking/ruby.rs

# godotenv just gives an error for that
# TODO: compose-go dotenv? probably also errors

pushd java
gradle -q run --args="--file ../quote-backtracking.env node ../dumpenv.js ${keys[*]}" > ../../quote_backtracking/java.rs
popd

pushd ../..
cargo run -- --strict=false --file=tests/generate/quote-backtracking.env node tests/generate/dumpenv.js "${keys[@]}" > tests/quote_backtracking/punktum.rs
popd

# escapes.env
# ===========

keys=(BASIC BACKSLASH QUOTES SINGLE_QUOTED1 SINGLE_QUOTED2 OCT1 OCT2 OCT3 OCT4 HEX UTF16 UTF16_PAIR UTF32_6 UTF32_8 NAMED1 NAMED2 NAMED3 UNKNOWN ESCAPED_NEWLINE)

node --env-file=escapes.env gen_dotenv.js "${keys[@]}" > ../escapes/nodejs.rs

DOTENV_CONFIG_PATH=escapes.env node gen_dotenv.js "${keys[@]}" > ../escapes/javascript.rs
"$PYTHON_DOTENV_CLI" --dotenv escapes-python-cli.env node dumpenv.js "${keys[@]}" > ../escapes/python_cli.rs

python -m dotenv --file escapes.env run --no-override node dumpenv.js "${keys[@]}" > ../escapes/python.rs

"$RUBY_DOTENV" -f escapes.env node dumpenv.js "${keys[@]}" > ../escapes/ruby.rs
DOTENV_LINEBREAK_MODE=legacy "$RUBY_DOTENV" -f escapes.env node dumpenv.js "${keys[@]}" > ../escapes/ruby_legacy.rs

"$GO_DOTENV" -f escapes-godotenv.env node dumpenv.js "${keys[@]}" > ../escapes/godotenv.rs
# TODO: compose-go dotenv

#dotenvy --file=escapes.env node dumpenv.js "${keys[@]}" > ../escapes/dotenvy.rs

pushd java
gradle -q run --args="--file ../escapes.env node ../dumpenv.js ${keys[*]}" > ../../escapes/java.rs
popd

pushd ../..
cargo run -- --strict=false --file=tests/generate/escapes.env node tests/generate/dumpenv.js "${keys[@]}" > tests/escapes/punktum.rs
popd
