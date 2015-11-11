extern crate bsh_rs;

use bsh_rs::parse::ParseInfo;
use bsh_rs::shell::Shell;
use std::process;

static HISTORY_CAPACITY: usize = 10;

fn main() {
    let mut shell = Shell::new(HISTORY_CAPACITY);
    loop {
        shell.check_jobs();
        let mut input = String::new();
        match Shell::prompt(&mut input) {
            Ok(0) => {
                println!("exit");
                process::exit(0);
            }
            Err(_) => panic!("failed to read line."),
            _ => {}
        }
        let mut info = match ParseInfo::parse(&input) {
            Ok(Some(info)) => info,
            Err(err) => {
                println!("{:?}", err);
                continue;
            }
            _ => continue,
        };
        shell.add_history(&info);

        if let Err(e) = shell.run(&mut info) {
            println!("bsh: {}", e);
        }
    }
}
