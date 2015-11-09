extern crate bsh_rs;

use bsh_rs::parse::ParseInfo;
use bsh_rs::shell::Shell;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::process::{Stdio};
use std::process;

fn execute_job(job: &mut ParseInfo) -> Result<(), io::Error> {
    let mut command = job.commands.get_mut(0).unwrap();
    // if it's a builtin, call the builtin

    if let Some(_) = job.infile {
        command.stdin(Stdio::piped());
    }

    if let Some(_) = job.outfile {
        command.stdout(Stdio::piped());
    }

    let mut child = try!(command.spawn());
    if let Some(ref mut stdin) = child.stdin {
        let infile = job.infile.take().unwrap();
        let mut f = try!(File::open(infile));
        let mut buf: Vec<u8> = vec![];
        try!(f.read_to_end(&mut buf));
        try!(stdin.write_all(&buf));
    }
    if let Some(ref mut stdout) = child.stdout {
        let outfile = job.outfile.take().unwrap();
        let mut file = try!(OpenOptions::new().write(true).create(true).open(outfile));
        let mut buf: Vec<u8> = vec![];
        try!(stdout.read_to_end(&mut buf));
        try!(file.write_all(&buf));
    } else {
        print!("{}",
                 String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout));
    }
    Ok(())
}

fn main() {
    loop {
        let mut input = String::new();
        match Shell::prompt(&mut input) {
            Ok(0) => {
                println!("exit");
                process::exit(0);
            },
            Err(_) => panic!("failed to read line."),
            _ => {},
        }
        let mut info = match ParseInfo::parse(&input) {
            Ok(Some(info)) => info,
            Err(err) => {
                println!("{:?}", err);
                continue;
            }
            _ => continue,
        };
        println!("parsed info: {:?}", info);

        if let Err(e) = execute_job(&mut info) {
            println!("bsh: {}", e);
        }
    }
}
