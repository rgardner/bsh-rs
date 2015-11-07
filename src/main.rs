#![feature(path_relative_from)]

extern crate bsh_rs;
use bsh_rs::parse::ParseInfo;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process;

fn prompt(buf: &mut String) -> io::Result<usize> {
    let cwd = env::current_dir().unwrap();
    let home = env::home_dir().unwrap();
    let rel = match cwd.relative_from(&home) {
        Some(rel) => Path::new("~/").join(rel),
        None => cwd.clone(),
    };

    print!("{} $ ", rel.display());
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
