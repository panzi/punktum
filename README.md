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
| NodeJS  | Works | Should be compatible to [NodeJS](https://nodejs.org/) v22's built in `--env-file=...` option. |
| PythonDotenvCLI | Works | Compatible to [dotenv-cli](https://github.com/venthur/dotenv-cli#readme) pypi pacakge |
| GoDotenv | WIP/Broken | Compatible to [compose-go/dotenv](https://github.com/compose-spec/compose-go/tree/main/dotenv) as use in docker-compose |
| JavaScriptDotenv | Not Implemented | Compatible to [dotenv](https://github.com/motdotla/dotenv#readme) npm package |
