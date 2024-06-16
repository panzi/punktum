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

This also comes with an [executable](#punktum-executable) that can be used
as a program starter that sets the environment of a process. Also see that
section for a description of configuration environment variables that are
read by both the library and the executable.

Dialects
--------

Of course no guarnatee is made that anything actually works. This is just
with my limited manual test.

| Dialect | Status | Description |
|:-|:-:|:-|
| [Punktum](#punktum-dialect) | Works | Crazy dialect I made up. More details below. |
| [NodeJS](#nodejs-dialect) | Works | Compatible to [NodeJS](https://nodejs.org/) v22's built-in `--env-file=...` option. The parser changed between NodeJS versions. |
| [PythonDotenvCLI](#python-dotenv-cli-dialect) | Works | Compatible to the [dotenv-cli](https://github.com/venthur/dotenv-cli) pypi package. There seem to be encoding errors in the Python version? Interpreting UTF-8 as ISO-8859-1? |
| ComposeGo | Works? | Compatible to the [compose-go/dotenv](https://github.com/compose-spec/compose-go/tree/main/dotenv) as use in docker-compose, but needs more testing. Well, even more than the others. |
| GoDotenv | Works | Compatible to [godotenv](https://github.com/joho/godotenv). This seems like a predecessor to the above. There are many things that aren't or aren't correctly handled by this that are better handeled by the docker-compose version. Both suffer from problems that arise from variable substitution being distinct from string literal and escape sequence parsing and by cheaping out by using regular expressions. |
| [RubyDotenv](#ruby-dotenv-dialect) | Works | Compatible to the [dotenv](https://github.com/bkeepers/dotenv) Ruby gem. The two above each claim to be compatible to this, but clearly at least one of them is wrong. **NOTE:** Command `$()` support is deliberately not implemented. I deem running programs from a `.env` file to be dangerous. Use a shell script if you want to do that. |
| [JavaScriptDotenv](#javascript-dotenv-dialect) | Works | Compatible to the [dotenv](https://github.com/motdotla/dotenv) npm package. The NodeJS dialect is meant to be the same as this, but of course isn't. |
| JavaDotenv | *Not Implemented* | Compatible to [java-dotenv](https://github.com/cdimascio/dotenv-java). Yet again subtly different. |
| Dotenvy | *Not Implemented* | Probably won't implement [dotenvy](https://github.com/allan2/dotenvy) support, since it is already a Rust crate. And it is a good dialect with a sane parser and at a glance comprehensive looking tests. **Use that!** |
| [Binary](#binary-dialect) | Works | Another silly dialect I made up. Records are always just `KEY=VALUE\0` (i.e. null terminated, since null cannot be in environment variables anyway). It ignores any encoding setting and only uses UTF-8. |

I might not implement any more dialects than I have right now.

Punktum Dialect
---------------

Details might change!

If `DOTENV_CONFIG_STRICT` is set to `false` (default is `true`) all sorts of
syntax errors are forgiven. Even if there are encoding errors parsing resumes in
the next line. While the implementations of all dialects are somewhat respecting
this setting only Punktum is resuming decoding on the next line, since it was
implemented as a line based parser, reading one line at a time from the file.

### Examples

```bash
# comment line
VAR1=BAR # comment after the value
VAR2=BAR# no need for a space before the #
VAR3="BAR" # this comment is handled correctly even though it ends with "
VAR4="BAR" "BAZ" # produces: "BAR BAZ"

WHITESPACE1=  spaces around the value are ignored  
WHITESPACE2=  but  between  the  words  spaces  are  preserved

MULTILINE="
  a multiline comment
  spanning several
  lines
  # not a comment
"

VARIABLE_SUBSTITUTIONS1="
  only in unquoted and double quoted values
  normal: $VAR1
  in braces: X${VAR1}X
  ${VAR1:?error message if \$VAR1 is empty or not set}
  default value: ${VAR1:-$OTHER}
  for more see below
"

VARIABLE_SUBSTITUTIONS2=${FOO:-
  variable substitutions
  can of course also be
  multiline, even without
  double quotes
}

ESCAPES="
  only in double quoted values
  no newline: \
  newline: \n
  carrige return: \r
  tab: \t
  backslash: \\
  dollar: \$
  unicode: \u00e4
  for more see below
"

RAW_STRING1='
  these escapes are taken verbatim: \n \t \\
'

# to write a value with single quotes and otherwise as a raw string:
RAW_STRING2='You cant'"'"'t fail!'

# explicitly import variables from the parent environment:
PATH
HOME
PWD
SHELL

# only then you can re-use them
SOME_PATH=$HOME/.env

# export keywords are ignored, the line is parsed as if there where no export:
export EXPORT_IGNORED=FOO BAR
```

### Syntax Definition

DOS line endings (`\r\n`) are converted to Unix line endings (`\n`) before
parsing, but single carrige returns (`\r`) are left as-is.

```plain
PUNKTUM       := { { WS } [ VAR_ASSIGN | VAR_IMPORT ] { WS } [ COMMENT ] ( "\n" | EOF ) }
VAR_ASSIGN    := NAME { WS } "=" { WS } [ VALUE ]
VAR_IMPORT    := NAME
NAME          := NAME_CHAR { NAME_CHAR }
NAME_CHAR     := "a"..."z" | "A"..."Z" | "0"..."9" | "_"
VALUE         := { DOUBLE_QUOTED | SINGLE_QUOTED | UNQUOTED }
DOUBLE_QUOTED := '"' { ESCAPE_SEQ | NOT('"' | "\" | "$") | VAR_SUBST } '"'
SINGLE_QUOTED := "'" { NOT("'") } "'"
UNQUOTED      := { NOT('"' | "'" | "$" | "\n" | "#") | VAR_SUBST }
VAR_SUBST     := "$" NAME | "${" NAME [ ( ":?" | "?" | ":-" | "-" | ":+" | "+" ) VALUE ] "}"
ESCAPE_SEQ    := "\" ( "\" | '"' | "'" | "$" | "r" | "n" | "t" | "f" | "b" | "\n" ) |
                 UTF16_ESC_SEQ | UTF32_ESC_SEQ
UTF16_ESC_SEQ := "\u" HEX*4
UTF32_ESC_SEQ := "\U" HEX*6
WS            := "\t" | "\x0C" | "\r" | " "
COMMENT       := "#" { NOT("\n") }
```

A single name without `=` imports the value from the parent environment. This way
you can e.g. use the `punktum` command with the `--replace` option to create a whole
new environemnt, but still explicitely use certain environment variables from the
system environment.

A value consists of a sequence of quoted and unquoted strings.

If not quoted, spaces around a value are trimmed. A comment starts with `#` even
if it touches a word on its left side.

Both single and double quoted strings can be multiline. Variables can be referenced
in unquoted and double quoted strings. Escape sequences are only evaluated inside
of double quoted strings. (Should they be also evaluated in unquoted values?)

Note that UTF-16 escape sequences need to encode valid surrogate pairs if they
encode a large enough code-point. Invalid Unicode values are rejected as an error.

The variable substitution syntax is similar to the Unix shell. Variables are only
read from the current environment, not the parent environemnt. You need to import
them first to use them. (Should that be changed?)

| Syntax | Description |
|:-|:-|
| `$VAR` or `${VAR}` | Empty string if unset. |
| `${VAR:?MESSAGE}` | Error if `$VAR` is empty or unset. If provided `MESSAGE` will be printed as the error message. |
| `${VAR?MESSAGE}` | Error if `$VAR` is unset. If provided `MESSAGE` will be printed as the error message. |
| `${VAR:-DEFAULT}` | Use `DEFAULT` if `$VAR` is empty or unset. |
| `${VAR-DEFAULT}` | Use `DEFAULT` if `$VAR` is unset. |
| `${VAR:+DEFAULT}` | Use `DEFAULT` if `$VAR` is not empty. |
| `${VAR+DEFAULT}` | Use `DEFAULT` if `$VAR` is set. |

The `MESSAGE`/`DEFAULT` part can be anything like in a value, only not a `}` outside
of a quoted string. (Maybe I should add `\{` and `\}` escapes?)

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

The fllowing also works for the Punktum dialect:

```JavaScript
var env = new Map();
// env is filled somehow...
for (const [key, value] of env) {
    console.log(`${key}=${JSON.stringify(value).replaceAll('$', '\\u0024')}`);
}
```

This should also work with Python's [dotenv-cli](https://github.com/venthur/dotenv-cli),
but the other dialects don't support UTF-16 Unicode escape sequences (`\u####`).

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

NodeJS Dialect
--------------

Based on the [dotenv parser](https://github.com/nodejs/node/blob/v22.x/src/node_dotenv.cc)
of NodeJS v22. After [complaining about some of these quirks](https://github.com/nodejs/node/issues/53461)
they said they'll fix it. Meaning once this is done this dialect needs to be
adapted again. Making myself more work. 🤦

### Quirks

This is meant to be compatible to the [JavaScript Dotenv Dialect](#javascript-dotenv-dialect),
but isn't.

While this dialect does support quoted values if there is any space between the
`=` and `"` it will not parse it as a quoted value, meaning the quotes will be
included in the output. I.e. this in `.env`:

```dotenv
FOO= "BAR"
```

Is equivalent to this in JSON:

```JSON
{ "FOO": "\"BAR\"" }
```

This dialect supports strings quoted in double quotes (`"`), single quotes (`'`)
and back ticks (`` ` ``). These strings can be multi-line, but only in double quoted
strings `\n` will be translated to newlines.

If the second quote is missing only the current line is used as the value for
the variable. Parsing of more variables continues in the next line!

Comments start with `#`. There doesn't need to be a space before the `#`.

Keys may contain *anything* except spaces (` `), including tabs and newlines.
Meaning this:

```dotenv
FOO#=1
BAR
=2
```

Is equivalent with this JSON:

```JSON
{ "FOO#": "1", "BAR\n": "2" }
```

Lines with syntax errors (i.e. no `=`) are silently ignored, but they will trip
up the parser so that the following correct line is also ignored.

Leading `export ` will be ignored. Yes, the `export` needs to be followed by a
space. If its a tab its used as part of the key.

Accepts `\n`, `\r\n`, and even singular `\r` as line seperator by
replacing `/\r\n?/` with `\n`. Meaning if you have a single carrige
return or DOS line ending in a quoted string it will be replaced by a
single newline.

JavaScript Dotenv Dialect
-------------------------

Based this version on [main.js](https://github.com/motdotla/dotenv/blob/8ab33066f90a20445d3c41e4fafba6c929c5e1a5/lib/main.js)
from the dotenv npm package.

### Quirks

This dialect supports strings quoted in double quotes (`"`), single quotes (`'`)
and back ticks (`` ` ``). These strings can be multi-line, but only in double
quoted strings `\n` and `\r` will be translated to newlines and carrige returns.

It doesn't process any other escape sequences, even though the regular expression
used to match quoted strings implies the existence of `\"`, `\'`, and `` \` `` in
the respective quoted stirngs. If a value does not match such a quoted string
literal correctly it will be interpreted as an unquoted string and only be read
to the end of the line.

However, later the decision on whether to replace `\r` and `\n` is made by simply
checking if the first character of the matched string was a double quote, not if
the double quote kind of regular expression had matched. Similarly the quotes
around a value are stripped if the first and last character are matching quotes,
again not if the reqular expression (that has the not processed escaped quote in
it) had matched.

Instead of `=` this dialect also accepts `:`, but only if there is no
space between it and the variable name.

A comment starts with `#` even if it touches a word on its left side.

The way the used regular expression works means that there can be a newline
between the varialbe name and `=`. Meaning this:

```dotenv
FOO
=BAR
```

Is equivalent to this JSON:

```JSON
{ "FOO": "BAR" }
```

This also means that this is parsed the same even though one might expect
it to be the variable is set check syntax:

```dotenv
export FOO
=BAR
```

Lines with syntax errors (i.e. no `=`) are silently ignored, but in contrast to
the NodeJS dialect it won't trip up the parser and the next line is correctly
parsed (if it doesn't have have syntax error itself).

Accepts `\n`, `\r\n`, and even singular `\r` as line seperator by
replacing `/\r\n?/` with `\n`. Meaning if you have a single carrige
return or DOS line ending in a quoted string it will be replaced by a
single newline.

Leading `export` will be ignored. The `export` and the following variable name
can be separated by any kind of white space.

Accepts `.` and `-` in addition to `a`...`z`, `A`...`Z`, and `0`...`9` as
part of variable names.

Ruby Dotenv Dialect
-------------------

Based on this version of [parser.rb](https://github.com/bkeepers/dotenv/blob/27c80ed122f9bbe403033282e922d74ca717d518/lib/dotenv/parser.rb)
and [substitution/variable.rb](https://github.com/bkeepers/dotenv/blob/27c80ed122f9bbe403033282e922d74ca717d518/lib/dotenv/substitutions/variable.rb) of the dotenv Ruby gem.
Command substitution is deliberately not implemented.

### Quirks

This dialect supports variable and command substitution. (The latter
deliberately not implemented in Punktum.) Command substitution is problematic
on its own, but the way it is implemented in Ruby dotenv is especially
problematic since its done in two passes. First variable references like
`$FOO` and `${BAR}` are substituted. Then in the resulting string commands
like `$(rm -rf /)` are substituted. This means if any of the variables
contain command syntax in literal form it will be executed in the command
pass. It can even be split over multiple variables. E.g. this `.env` file:

```dotenv
FOO='$'
BAR='(date)'
BAZ="$FOO$BAR"
```

Used like this:

```bash
dotenv -f test.env ruby -e 'puts ENV.select {|k| %w(FOO BAR BAZ).include? k}'
```

Will give output like this:

```Ruby
{"FOO"=>"$", "BAR"=>"(date)", "BAZ"=>"Fr 14 Jun 2024 17:49:33 CEST"}
```

Personally I consider this as a code execution vulnerability. It is not
unthinkable that an environment variable used in a substitution contains
a string controlled by a user who injects a command this way. See
[this bug report](https://github.com/bkeepers/dotenv/issues/507).

Another minor quirk is that `{` and `}` in variable substitution don't
need to be balanced. `${FOO`, `$FOO}`, `${FOO}`, and `$FOO` all do the
same.

The dotenv file is parsed with a regular expression. Anything not
matching is simply silently ignored.

The regular expression handles escapes in when tokenizing the file
that way, but if the quoted string part of the regular expression
fails the no-quote part will still match. It is not checked how the
value was matched, only if it starts and ends in the same kind of
quote in order to determine how to process the value.

Quoted strings can be multiline. In double and single quoted
strings any backslashes of escape sequences are simply removed.
Meaning `\n` becomes `n`. However, if the environment variable
`DOTENV_LINEBREAK_MODE` is set to `legacy` (either in the currently
created environment or if it is unset there also the system
environment) then `\n` and `\r` in double quoted strings are
replaced with newlines and carrige returns.

Variable and command substitution is performed in double quoted and
non-quoted strings.

Lines in the form of `export FOO BAR BAZ` are interpreted as checking
if the listed keys exist in the environment. If not an error is raised.

Instead of `=` this dialect also accepts `:`, but only if there is no
space between it and the variable name.

Accepts `\n`, `\r\n`, and even singular `\r` as line seperator by
replacing `/\r\n?/` with `\n`. Meaning if you have a single carrige
return or DOS line ending in a quoted string it will be replaced by a
single newline.

Accepts `.` in addition to `a`...`z`, `A`...`Z`, `0`...`9`, and `_` as
part of variable names.

Python Dotenv-CLI Dialect
-------------------------

Based on this version of [core.py](https://github.com/venthur/dotenv-cli/blob/78ab62bfc33903eeee59ef483a8b1399c7d6dbd5/dotenv_cli/core.py)
of the dotenv-cli pypi package.

### Quirks

This dialect uses Python str functions for many things, and as such grammar
rules often derive from that. I.e. lines are split on `\r\n`, single `\n`,
and on single `\r`.

It only supports single line variables, because it parses one line at a time.

Comments start with `#` and must be on their own line (though can be preceeded
by white space)! Anything non-white space after a `=` is always part of the
variable value.

A key can contain anything except for `=` and white space around it will be
stripped. But you can do:

```dotenv
 FOO  BAR! = BAZ 
```

Equivalent to this in JSON:

```JSON
{ "FOO  BAR!": "BAZ" }
```

If a key starts with `export ` (yes, including the single space) this prefix and
any remaining heading white spaces are stripped. This of course means that
`export =foo` will lead to an empty string, which is an OS level error for
environment variable names, while `export=foo` is perfectly fine. For any other
variable name spaces between it and the `=` are insignifficant.

Values are also stripped of white space. If the remaining string starts or ends
with the same kind of quotes (either `"` or `'`) those quotes are removed. If
it's a double quoted string escape sequences are processed using this Python
code:

```Python
value = bytes(value, "utf-8").decode("unicode-escape")
```

Meaning which escape sequences are supported is defined by Python and might
change in a futher Python release!

The [Python documentation](https://docs.python.org/3/library/codecs.html#text-encodings)
says about this encoding:

> Encoding suitable as the contents of a Unicode literal in ASCII-encoded
> Python source code, except that quotes are not escaped. Decode from Latin-1
> source code. Beware that Python source code actually uses UTF-8 by default.

This leads to typical "UTF-8 interpreted as ISO-8859-1" errors for every double
quoted string! Completely breaks UTF-8 support for double quoted strings. This
dotenv file:

```dotenv
FOO=ä
BAR="ä"
BAZ='ä'
```

Is equivalent to this JSON:

```JSON
{ "FOO": "ä", "BAR": "Ã¤", "BAZ": "ä" }
```

For what escape sequences are actually supported see the
[Python documentation](https://docs.python.org/3/reference/lexical_analysis.html#escape-sequences).

**NOTE:** The Punktum implementation of this dialect doesn't do that. It treats
the string as the Unicode that it is.

**NOTE:** The Punktum implementation of this dialect doesn't implement *named*
Unicode escape sequences (`\N{name}`).

`punktum` Executable
--------------------

Punktum comes as a library and as a binary.

**NOTE:** On Windows (or any non-Unix operating system supported by Rust) there is no
`exec()` available. Meaning there is no way to replace the currently executing program
with another. So instead the command is spawned as a sub-process and it's exit code
is passed through at the end. However, forwarding things like Ctrl+C (or killing
sub-processes when the parent exits) is not straight forward under Windows. This would
need to be implemented with a lot of custom unsafe code calling Win32 functions, so I
didn't do it. This means if you kill the punktum process the child process will keep
running. I think. I haven't tested it under Windows, I use Linux.

Usage of the binary:

```plain
usage: punktum [--file=PATH...] [--replace] [--] command [args...]
       punktum [--file=PATH...] [--replace] --print-env [--sorted] [--export] [--binary]
       punktum [--help] [--version]

Punktum executes a given command with environment variables loaded from a .env file.

Positional arguments:
  command                   Program to execute.

Optional arguments:
  -h, --help                Print this help message and exit.
  -v, --version             Print program's version and exit.
  -f PATH, --file=PATH      File to use instead of ".env"
                            This option can be passed multiple times.
                            All files are loaded in order.
                            Pass "-" to read from stdin.
  -r, --replace             Completely replace the environment with the one loaded
                            from the .env file.
  -p, --print-env           Instead of running a command print the built environment
                            in a syntax compatible to Punktum and bash.
      --sorted              Sort printed environment variables for reproducible output.
      --export              Add "export " prefix to every printed environment variable.
      --strict=bool         Overwrite DOTENV_CONFIG_STRICT
      --debug=bool          Overwrite DOTENV_CONFIG_DEBUG
      --override=bool       Overwrite DOTENV_CONFIG_OVERRIDE
      --encoding=ENCODING   Overwrite DOTENV_CONFIG_ENCODING
      --dialect=DIALECT     Overwrite DOTENV_CONFIG_DIALECT

Environemnt variables:
  DOTENV_CONFIG_PATH=FILE
    File to use instead of ".env".
    This can be overwritten with --file.
    [default: ".env"]

  DOTENV_CONFIG_STRICT=true|false
    Stop and return an error if any problem is encounterd,
    like a file is not found, an encoding error, or a syntax error.
    This can be overwritten with --strict.
    [default: true]

  DOTENV_CONFIG_DEBUG=true|false
    Write debug messages to stderr if there are any problems.
    This can be overwritten with --debug.
    [default: false]

  DOTENV_CONFIG_OVERRIDE=true|false
    Replace existing environment variables.
    This can be overwritten with --override.
    [default: false]

  DOTENV_CONFIG_ENCODING=ENCODING
    Encoding of ".env" file.
    This can be overwritten with --encoding.
    [default: UTF-8]

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
    This can be overwritten with --dialect.
    [default: Punktum]

    Supported values:
    - Punktum (default)
    - NodeJS
    - PythonDotenvCLI
    - ComposeGo
    - GoDotenv
    - RubyDotenv
    - Binary

  DOTENV_LINEBREAK_MODE=legacy
    RubyDotenv dialect-only. If this environment variable is set to "legacy"
    "\n" and "\r" in unquoted values and double quoted values are replaced
    with actual newline and carrige return characters.
```
