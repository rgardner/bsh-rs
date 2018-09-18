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
