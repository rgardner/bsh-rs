extern crate bsh_rs;

use bsh_rs::parse::ParseInfo;
use std::io::{self, Write};
use std::process;

fn prompt(buf: &mut String) -> io::Result<usize> {
    print!("$ ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(buf)
}

fn main() {
    loop {
        let mut input = String::new();
        if let Ok(n) = prompt(&mut input) {
            if n == 0 {
                println!("");
                process::exit(0);
            }
        } else {
            panic!("failed to read line.");
        }
        let command = match ParseInfo::parse(&input) {
            Ok(Some(info)) => info,
            Err(err) => {
                println!("{:?}", err);
                continue
            }
            _ => continue,
        };
        println!("parsed command: {:?}", command);
    }
}
