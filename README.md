# bsh-rs
This is the [Rust](https://www.rust-lang.org/) version of my
[bsh](htttps://github.com/rgardner/bsh) program. This program uses unstable
features (see the [unstable](#unstable) section below) and thus requires Rust
nightly to compile.

## Goals
* learn idiomatic Rust
* make the C version of `bsh` more memory safe by using Rust's memory safety
  principles
* contribute back to the Rust compiler

## Unstable Features
Rust prohibits the use of unstable features on the stable compiler, which is
why `bsh-rs` uses the nightly compiler. The following unstable features are
used:

- `path_relative_from`
  + This is a helper method to make it easier for the prompt to show the
    current working directory relative to home when it's in a child directory
    and to otherwise show the full path. e.g. it will show `/usr/local/bin` and
    `~/Desktop`.
