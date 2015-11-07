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
                println!("exit");
                process::exit(0);
            }
        } else {
            panic!("failed to read line.");
        }
        let info = match ParseInfo::parse(&input) {
            Ok(Some(info)) => info,
            Err(err) => {
                println!("{:?}", err);
                continue;
            }
            _ => continue,
        };
        println!("parsed info: {:?}", info);

        for mut command in info.commands {
            let output = match command.output() {
                Ok(output) => output,
                Err(e) => {
                    println!("bsh: {}", e);
                    continue;
                }
            };
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }
}
