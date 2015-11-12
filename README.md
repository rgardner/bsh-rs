# bsh-rs
[![Build Status](https://magnum.travis-ci.com/rgardner/bsh-rs.svg?token=PKiUsiwCCXnqdp7dRvmq&branch=master)](https://magnum.travis-ci.com/rgardner/bsh-rs)

This is the [Rust](https://www.rust-lang.org/) version of my [bsh
shell program](https://github.com/rgardner/bsh).

This program supports conditional compilation with unstable features that
require the nightly compiler.  See the [unstable](#unstable-features) section
below for more information.


## Installation
```bash
$ git clone https://github.com/rgardner/bsh-rs
$ cd bsh-rs
$ # run stable
$ cargo run
$ # run with unstable features, requires the nightly compiler
$ cargo run --features 'unstable'
```


## Goals
* learn idiomatic Rust
* make the C version of `bsh` more memory safe by using Rust's memory safety
  principles
* contribute back to the Rust compiler


## Unstable Features
Rust prohibits the use of unstable features on the stable compiler, which is
why `bsh-rs` requires the nightly compiler to build with unstable features.

To build with unstable features, invoke cargo with `--features 'unstable'`

- `path_relative_from`
  + This is a helper method to make it easier for the prompt to show the
    current working directory relative to home when it's in a child directory
    and to otherwise show the full path. e.g. it will show `/usr/local/bin` and
    `~/Desktop`.
