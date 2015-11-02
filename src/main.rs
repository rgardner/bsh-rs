extern crate bsh_rs;

use bsh_rs::parse;
use std::io::{self, Write};

fn prompt(buf: &mut String) {
    print!("$ ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(buf).ok().expect("failed to read line");
}

fn main() {
    loop {
        let mut input = String::new();
        prompt(&mut input);
        let command = parse::parse(&input);
        println!("parsed command: {:?}", command);
    }
}
