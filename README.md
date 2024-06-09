punktum
=======

Yet another dotenv implementation for Rust. Just for fun. Don't use it, it
won't be maintained. You may fork it if you want to.

"Punkt" is the German word for "dot" and "Umgebung" means "environment".

**Work in progress!**

I'm trying to implement multiple dotenv dialects with mixed success. Also
so far I don't have any dependencies and like to keep it that way, which
might be a problem for certain dialects that use complex regular
expressions.

Dialects
--------

Of course no guarnatee is made that anything actually works. This is just
with my limited manual test.

| Dialect | Status | Description |
|:-|:-:|:-|
| Punktum | Works | Crazy dialect I made up. |
| NodeJS  | Works | Compatible to [NodeJS](https://nodejs.org/) v22's built-in `--env-file=...` option. The parser changed between NodeJS versions. |
| PythonDotenvCLI | Works | Compatible to the [dotenv-cli](https://github.com/venthur/dotenv-cli#readme) pypi package. |
| ComposeGo | Works? | Compatible to the [compose-go/dotenv](https://github.com/compose-spec/compose-go/tree/main/dotenv) as use in docker-compose, but needs more testing. Well, even more than the others. |
| GoDotenv | Not Implemented | Compatible to [godotenv](https://github.com/joho/godotenv), which is slightly different to the above. |
| RubyDotenv | Not Implemented | Compatible to the [dotenv](https://github.com/bkeepers/dotenv) Ruby gem. The two above each claim to be compatible to this, but clearly at least one of them is wrong. |
| JavaScriptDotenv | Not Implemented | Compatible to the [dotenv](https://github.com/motdotla/dotenv#readme) npm package. |

I might not implement any more dialects than I have right now.

**TODO:** Describe Punktum dialect.

Binary
------

Punktum comes as a library and as a binary. Usage of the binary:

```plain
usage: punktum [--file=DOTENV...] [--replace] [--] command [args...]
       punktum [--file=DOTENV...] [--replace] --print-env [--sorted] [--export]
       punktum [--help] [--version]

Punktum executes a given command with environment variables loaded from a .env file.

Positional arguments:
  command                   Program to execute.

Optional arguments:
  -h, --help                Print this help message and exit.
  -v, --version             Print program's version and exit.
  -f DOTENV, --file=DOTENV  File to use instead of .env
                            This option can be passed multiple times. All files are loaded in order.
  -r, --replace             Completely replace all existing environment variables with the ones loaded from the .env file.
  -p, --print-env           Instead of running a command print the built environment in a syntax compatible to Punktum and bash.
      --sorted              Sort printed environment variables for reproducible output.
      --export              Add 'export ' prefix to every printed environment variable.

Environemnt variables:
  DOTENV_CONFIG_PATH=FILE  (default: .env)
    File to use instead of .env. This can be overwritten by --file.

  DOTENV_CONFIG_STRICT=true|false  (default: true)
    Stop and return an error if any problem is encounterd,
    like a file is not found, an encoding error, or a syntax error.

  DOTENV_CONFIG_DEBUG=true|false  (default: false)
    Write debug messages to stderr if there are any problems.

  DOTENV_CONFIG_OVERRIDE=true|false  (default: false)
    Replace existing environment variables.

  DOTENV_CONFIG_ENCODING=ENCODING
    Encoding of .env file.

    Supported values:
    - ASCII
    - ISO-8859-1 (alias: Latin1)
    - UTF-8 (default)
    - UTF-16BE
    - UTF-16LE
    - UTF-32BE
    - UTF-32LE

  DOTENV_CONFIG_DIALECT=DIALECT
    Dialect for the parser to use.

    Supported values:
    - Punktum (default)
    - NodeJS
    - PythonDotenvCLI
    - ComposeGo
```
