# bsh-rs
[![Build Status](https://magnum.travis-ci.com/rgardner/bsh-rs.svg?token=PKiUsiwCCXnqdp7dRvmq&branch=master)](https://magnum.travis-ci.com/rgardner/bsh-rs)

This is the [Rust](https://www.rust-lang.org/) version of my [bsh
shell program](https://github.com/rgardner/bsh).


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
$ # build
$ cargo build
$ # run tests
$ cargo test
$ # generate documentation
$ cargo doc
$ # run clippy lints via features
$ rustup run +"$(cat rustnightly.txt)" cargo build --features "clippy"
$ # run clippy lints via cargo subcommand
$ rustup run +"$(cat rustnightly.txt)" cargo clippy
```

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
