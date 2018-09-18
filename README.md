# bsh-rs
This is the [Rust](https://www.rust-lang.org/) version of my [bsh
shell program](https://github.com/rgardner/bsh).


## Features

* runs builtin and external commands
* expands history and environment variables
* supports `|`, `;`, `||`, `&&`
* supports job control
* has the following builtins:
  - `cd`
  - `history`
  - `kill`
  - `exit`
  - `bg`, `fg`, `jobs`
  - `declare`, `unset`


## Goals

* learn idiomatic Rust
* make the C version of `bsh` more memory safe by using Rust's memory safety
  principles
* contribute back to the Rust ecosystem
  - [rustyline: Add `const` way of accessing editor history](https://github.com/kkawakam/rustyline/commit/f536c969e73bb121a3968b71342db5dba4e885fa)
  - [rustyline: Fix multi-line prompts clearing too many lines](https://github.com/kkawakam/rustyline/commit/59c4b7b045870127405da4ef8345cd917740166f)
  - [nix (in-progress): add wrapper for signal(3) function](https://github.com/nix-rust/nix/pull/817)


## Usage

```sh
$ bsh --help
bsh.

Usage:
    bsh [options]
    bsh [options] -c <command>
    bsh [options] <file>
    bsh (-h | --help)
    bsh --version

Options:
    -h --help       Show this screen.
    --version       Show version.
    -c              If the -c option is present, then commands are read from the first non-option
                        argument command_string.
    --log=<path>    File to write log to, defaults to ~/.bsh_log
$ bsh
0|~/code
$ help
bg: bg [<jobspec>...]
cd: cd [dir]
declare: declare [name[=value] ...]
exit: exit [n]
fg: fg [job_spec]
help: help [command ...]
history: history [-c] [-s size] [n]
jobs: jobs [options] [<jobspec>...]
kill: kill pid | %jobspec
unset: unset [name ...]
```

## Installation

```bash
$ git clone https://github.com/rgardner/bsh-rs
$ cd bsh-rs
$ # run bsh via cargo
$ cargo run
$ # run bsh directly
$ target/debug/bsh
$ # display bsh help
$ target/debug/bsh --help
```


## Development

```bash
$ # setup dev environment (e.g. git hooks)
$ ./scripts/dev_setup.sh
$ # Check program for errors, quicker than full build
$ cargo check
$ # build
$ cargo build
$ # run lints
$ cargo clippy
$ # run tests
$ cargo test
$ # generate documentation
$ cargo doc --document-private-items
```


## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
