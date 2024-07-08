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
INHERIT=inherited compose-go/dotenv --file edge-cases-composego.env node dumpenv.js > ../edge_cases/composego.rs

# It generates a variable with name "" and value "inherited", which is discarded by the os,
# since there can't be zero-length environment variable names.
#INHERIT=inherited compose-go/dotenv --file inherit-eof.env --replace node dumpenv.js INHERIT > ../edge_cases/composego_inherit_eof.rs

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

# godotenv and compose-go just give an error for that

pushd java
gradle -q run --args="--file ../quote-backtracking.env node ../dumpenv.js ${keys[*]}" > ../../quote_backtracking/java.rs
popd

pushd ../..
cargo run -- --strict=false --file=tests/generate/quote-backtracking.env node tests/generate/dumpenv.js "${keys[@]}" > tests/quote_backtracking/punktum.rs
popd

# escapes.env
# ===========

keys=(BASIC BACKSLASH QUOTES SINGLE_QUOTED1 SINGLE_QUOTED2 INVALID_OCT OCT1 OCT2 OCT3 OCT4 HEX UTF16 UTF16_PAIR UTF32_6 UTF32_8 NAMED1 NAMED2 NAMED3 UNKNOWN ESCAPED_NEWLINE)

node --env-file=escapes.env gen_dotenv.js "${keys[@]}" > ../escapes/nodejs.rs

DOTENV_CONFIG_PATH=escapes.env node gen_dotenv.js "${keys[@]}" > ../escapes/javascript.rs
"$PYTHON_DOTENV_CLI" --dotenv escapes-python-cli.env node dumpenv.js "${keys[@]}" > ../escapes/python_cli.rs

python -m dotenv --file escapes.env run --no-override node dumpenv.js "${keys[@]}" > ../escapes/python.rs

"$RUBY_DOTENV" -f escapes.env node dumpenv.js "${keys[@]}" > ../escapes/ruby.rs
DOTENV_LINEBREAK_MODE=legacy "$RUBY_DOTENV" -f escapes.env node dumpenv.js "${keys[@]}" > ../escapes/ruby_legacy.rs

"$GO_DOTENV" -f escapes-godotenv.env node dumpenv.js "${keys[@]}" > ../escapes/godotenv.rs
compose-go/dotenv --file escapes.env node dumpenv.js "${keys[@]}" > ../escapes/composego.rs

#dotenvy --file=escapes.env node dumpenv.js "${keys[@]}" > ../escapes/dotenvy.rs

pushd java
gradle -q run --args="--file ../escapes.env node ../dumpenv.js ${keys[*]}" > ../../escapes/java.rs
popd

pushd ../..
cargo run -- --strict=false --file=tests/generate/escapes.env node tests/generate/dumpenv.js "${keys[@]}" > tests/escapes/punktum.rs
popd

# varsubst.env
# ============

keys=(UNSET VAR1 VAR2 VAR3 VAR4 VAR5 VAR6 VAR7)

python -m dotenv --file varsubst.env run --no-override node dumpenv.js "${keys[@]}" > ../varsubst/python.rs
compose-go/dotenv --file varsubst.env node dumpenv.js "${keys[@]}" > ../varsubst/composego.rs
"$RUBY_DOTENV" -f varsubst.env node dumpenv.js "${keys[@]}" > ../varsubst/ruby.rs

pushd ../..
cargo run -- --strict=false --file=tests/generate/varsubst.env node tests/generate/dumpenv.js "${keys[@]}" > tests/varsubst/punktum.rs
popd

# varsubst-ext.env
# ================

keys=(EMPTY FOO UNSET VAR1S VAR1E VAR1U VAR2 VAR3S VAR3E VAR3U VAR4S VAR4E VAR4U VAR5S VAR5E VAR5U VAR6 VAR7 VAR8)

python -m dotenv --file varsubst-ext.env run --no-override node dumpenv.js "${keys[@]}" > ../varsubst_ext/python.rs
compose-go/dotenv --file varsubst-ext.env node dumpenv.js "${keys[@]}" > ../varsubst_ext/composego.rs

pushd ../..
cargo run -- --strict=false --file=tests/generate/varsubst-ext.env node tests/generate/dumpenv.js "${keys[@]}" > tests/varsubst_ext/punktum.rs
popd
