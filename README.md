# bsh-rs
[![Build Status](https://magnum.travis-ci.com/rgardner/bsh-rs.svg?token=PKiUsiwCCXnqdp7dRvmq&branch=master)](https://magnum.travis-ci.com/rgardner/bsh-rs)

This is the [Rust](https://www.rust-lang.org/) version of my [bsh
shell program](https://github.com/rgardner/bsh).


## Installation

```bash
$ git clone https://github.com/rgardner/bsh-rs
$ cd bsh-rs
$ # run stable
$ cargo run
$ # run clippy lints, requires the nightly compiler
$ cargo run --features clippy
```


## Goals

* learn idiomatic Rust
* make the C version of `bsh` more memory safe by using Rust's memory safety
  principles
* contribute back to the Rust compiler
