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
| PythonDotenvCLI | Works | Compatible to the [dotenv-cli](https://github.com/venthur/dotenv-cli) pypi package. There seem to be encoding errors in the Python version? Interpreting UTF-8 as ISO-8859-1? |
| ComposeGo | Works? | Compatible to the [compose-go/dotenv](https://github.com/compose-spec/compose-go/tree/main/dotenv) as use in docker-compose, but needs more testing. Well, even more than the others. |
| GoDotenv | Works | Compatible to [godotenv](https://github.com/joho/godotenv). This seems like a predecessor to the above. There are many things that aren't or aren't correctly handled by this that are better handeled by the docker-compose version. Both suffer from problems that arise from variable substitution being destinct from string literal and escape sequence parsing and by cheaping out by using regular expressions. |
| RubyDotenv | Works | Compatible to the [dotenv](https://github.com/bkeepers/dotenv) Ruby gem. The two above each claim to be compatible to this, but clearly at least one of them is wrong. |
| JavaScriptDotenv | Not Implemented | Compatible to the [dotenv](https://github.com/motdotla/dotenv) npm package. |
| Dotenvy | Not Implemented | Probably won't implement [dotenvy](https://github.com/allan2/dotenvy) support, since it is already a Rust crate. And it is a good dialect with a sane parser. Use that! |
| Binary | Works | Another silly dialect I made up. Records are always just `KEY=VALUE\0` (i.e. null terminated, since null cannot be in environment variables anyway). It ignores any encoding setting and only used UTF-8. |

I might not implement any more dialects than I have right now.

Punktum Dialect
---------------

Details might change.

```plain
PUNKTUM       := { ( VAR_ASSIGN | VAR_IMPORT ) "\n" }
VAR_ASSIGN    := NAME "=" [ VALUE ]
VAR_IMPORT    := NAME
NAME          := NAME_CHAR { NAME_CHAR }
NAME_CHAR     := "a"..."z" | "A"..."Z" | "0"..."9" | "_"
VALUE         := { DOUBLE_QUOTED | SINGLE_QUOTED | UNQUOTED }
DOUBLE_QUOTED := '"' { ESCAPE_SEQ | NOT('"') | VAR_SUBST } '"'
SINGLE_QUOTED := "'" { NOT("'") } "'"
UNQUOTED      := { NOT('"' | "'" | "$" | "\n" | "#") | VAR_SUBST }
VAR_SUBST     := "$" NAME | "${" NAME [ ":?" | "?" | ":-" | "-" | ":+" | "+" ] VALUE "}"
ESCAPE_SEQ    := "\" ( "\" | '"' | "'" | "$" | "r" | "n" | "t" | "f" | "b" | "\n" ) |
                 UTF16_ESC_SEQ | UTF32_ESC_SEQ
UTF16_ESC_SEQ := "\u" HEX*4
UTF32_ESC_SEQ := "\U" HEX*6
COMMENT       := "#" { NOT("\n") }
```

A single name without `=` imports the value from the parent environment. This way
you can e.g. use the `punktum` command with the `--replace` option to create a whole
new environemnt, but still explicitely use certain environment variables from the
system environment.

A value consists of a sequence of quoted and unquoted strings.

If not quoted, spaces around a value are trimmed. A comment starts with `#` even
if it touches a word on its left side.

Both single and double quoted strings can be multiline. Variables can be refrenced
in unquoted and double quoted strings. Escape sequences are only evaluated inside
of double quoted strings.

Note that UTF-16 escape sequences need to encode valid surrogate pairs if they
encode a large enough code-point. Invalid Unicode values are rejected as an error.

The variable substitution syntax is similar to the Unix shell. Variables are only
read from the current environment, not the parent environemnt. You need to import them
first to use them. (Should that be changed?)

| Syntax | Description |
|:-|:-|
| `${VAR:?MESSAGE}` | Error if `$VAR` is empty or unset. If provided `MESSAGE` will be printed as the error message. |
| `${VAR?MESSAGE}` | Error if `$VAR` is unset. If provided `MESSAGE` will be printed as the error message. |
| `${VAR:-DEFAULT}` | Use `DEFAULT` if `$VAR` is empty or unset. |
| `${VAR-DEFAULT}` | Use `DEFAULT` if `$VAR` is unset. |
| `${VAR:+DEFAULT}` | Use `DEFAULT` if `$VAR` is not empty. |
| `${VAR+DEFAULT}` | Use `DEFAULT` if `$VAR` is set. |

The `MESSAGE`/`DEFAULT` part can be anything like in a value, only not a `}` outside
of a quoted string.

If you want to write a `.env` file in the Punktum dialect conatining arbitarary
characters you can quote the values very easily like this:

```JavaScript
var env = new Map();
// env is filled somehow...
for (const [key, value] of env) {
    console.log(`${key}='${value.replaceAll("'", "'\"'\"'")}'`);
}
```

Meaning, you put the value into single quotes and replace any `'` in your value
with `'"'"'`.

The keys need to be valid names as described above, though. This then happens to
also be valid Unix shell syntax and I think also valid syntax for
[dotenvy](https://github.com/allan2/dotenvy). It isn't valid for many (any?)
other dotenv implementations, since they only allow one single quoted string and
not a sequence of quoted strings.

This also works for the Punktum dialect, as long as there are no `$VARIABLE`
substitutions in your strings:

```JavaScript
var env = new Map();
// env is filled somehow...
for (const [key, value] of env) {
    console.log(`${key}=${JSON.stringify(value)}`);
}
```

It should work best with Python's [dotenv-cli](https://github.com/venthur/dotenv-cli),
but the other dialects don't support UTF-16 unicode escape sequences (`\u####`).

Binary Dialect
--------------

The *Binary* dialect as an output-format can be used for things like this:

```bash
punktum --replace --file examples/vars.env --sorted --print-env --binary | while read -r -d "" line; do
    printf "%s=%q\n" "${line%%=*}" "${line#*=}"
done
```

I don't know why you'd want to do that, but you can!

Writing it is also simple:

```Rust
let env = HashMap::new();
// env is filled somehow...
for (key, value) in &env {
  write!(writer, "{key}={value}\0")?;
}
```

`punktum` Binary
----------------

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
                            This option can be passed multiple times.
                            All files are loaded in order.
  -r, --replace             Completely replace all existing environment variables with
                            the ones loaded from the .env file.
  -p, --print-env           Instead of running a command print the built environment
                            in a syntax compatible to Punktum and bash.
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
    - ISO-8859-1  (alias: Latin1)
    - UTF-8       (default)
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
    - Binary
```
