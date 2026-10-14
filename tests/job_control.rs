//! Regression coverage using a real controlling terminal.

#![cfg(unix)]

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::fcntl::{FcntlArg, OFlag, fcntl};
use nix::pty::openpty;
use nix::unistd::{self, Pid};
use tempfile::TempDir;

struct TerminalShell {
    child: Child,
    terminal: File,
    directory: TempDir,
    shell_pgid: Pid,
}

impl TerminalShell {
    fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let pty = openpty(None, None).unwrap();
        // Take ownership so both terminal descriptors are closed on every path.
        let terminal = unsafe { File::from_raw_fd(pty.master) };
        let slave = unsafe { File::from_raw_fd(pty.slave) };
        let mut command = Command::new("/bin/sh");
        // The wrapper owns the session; bsh must be able to create its own
        // process group without being the session leader (setpgid forbids that).
        command
            .args(["-c", "\"$1\" --log shell.log; exit $?", "sh"])
            .arg(env!("CARGO_BIN_EXE_bsh"))
            .current_dir(directory.path())
            .env("HOME", directory.path())
            .env("TERM", "xterm")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave));
        unsafe {
            command.pre_exec(|| {
                unistd::setsid().map_err(io::Error::from)?;
                if libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY.into(), 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().unwrap();
        fcntl(terminal.as_raw_fd(), FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();
        let mut shell = Self {
            child,
            terminal,
            directory,
            shell_pgid: Pid::from_raw(0),
        };
        shell.wait_for_prompt();
        shell.shell_pgid = unistd::tcgetpgrp(shell.terminal.as_raw_fd()).unwrap();
        assert_ne!(shell.shell_pgid.as_raw(), shell.child.id() as i32);
        shell
    }

    fn send(&mut self, input: &str) {
        self.terminal.write_all(input.as_bytes()).unwrap();
    }

    fn wait_for_prompt(&mut self) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut output = Vec::new();
        loop {
            let mut buffer = [0; 4096];
            match self.terminal.read(&mut buffer) {
                Ok(0) => panic!(
                    "shell closed terminal: {}",
                    String::from_utf8_lossy(&output)
                ),
                Ok(count) => {
                    output.extend_from_slice(&buffer[..count]);
                    if output.windows(3).any(|bytes| bytes == b"\n$ ") {
                        return String::from_utf8_lossy(&output).into_owned();
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => panic!(
                    "terminal read failed: {error}: {}",
                    String::from_utf8_lossy(&output)
                ),
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for prompt: {}",
                String::from_utf8_lossy(&output)
            );
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "shell exited: {}",
                String::from_utf8_lossy(&output)
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn run(&mut self, command: &str) -> String {
        self.send(&format!("{command}\n"));
        let output = self.wait_for_prompt();
        assert_eq!(
            unistd::tcgetpgrp(self.terminal.as_raw_fd()).unwrap(),
            self.shell_pgid,
            "shell must own the terminal at the prompt"
        );
        output
    }
    fn run_with_input(&mut self, command: &str, input: &str) {
        self.send(&format!("{command}\n"));
        let deadline = Instant::now() + Duration::from_secs(5);
        while unistd::tcgetpgrp(self.terminal.as_raw_fd()).unwrap() == self.shell_pgid {
            // Drain the echoed command: leaving output pending can block the line
            // editor's TCSADRAIN restoration before the foreground process starts.
            let mut buffer = [0; 4096];
            match self.terminal.read(&mut buffer) {
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => panic!("terminal read failed: {error}"),
            }
            assert!(
                Instant::now() < deadline,
                "foreground job never took terminal control"
            );
            thread::sleep(Duration::from_millis(10));
        }
        self.send(input);
        self.wait_for_prompt();
        assert_eq!(
            unistd::tcgetpgrp(self.terminal.as_raw_fd()).unwrap(),
            self.shell_pgid
        );
    }
}

impl Drop for TerminalShell {
    fn drop(&mut self) {
        if self.shell_pgid.as_raw() > 0 {
            let _ = nix::sys::signal::kill(self.shell_pgid, nix::sys::signal::Signal::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn background_jobs_preserve_prompt_and_input() {
    let mut shell = TerminalShell::start();
    shell.run("sleep 1 &");
    shell.run("sleep 1 | cat &");

    // Unredirected background readers receive EOF instead of consuming input
    // intended for the prompt. Pipes and explicit redirects still supply data.
    shell.run("/bin/sh -c 'cat >empty; echo eof >reader-finished' &");
    fs::write(shell.directory.path().join("input"), "redirected input\n").unwrap();
    shell.run("cat <input >redirected &");
    shell.run("echo pipeline-input | cat >piped &");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !shell.directory.path().join("reader-finished").exists()
        || fs::read_to_string(shell.directory.path().join("redirected")).unwrap_or_default()
            != "redirected input\n"
        || fs::read_to_string(shell.directory.path().join("piped")).unwrap_or_default()
            != "pipeline-input\n"
    {
        assert!(
            Instant::now() < deadline,
            "background readers did not complete"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read(shell.directory.path().join("empty")).unwrap(), b"");
    assert_eq!(
        fs::read_to_string(shell.directory.path().join("redirected")).unwrap(),
        "redirected input\n"
    );
    assert_eq!(
        fs::read_to_string(shell.directory.path().join("piped")).unwrap(),
        "pipeline-input\n"
    );

    // Foreground readers must still receive terminal input and return ownership
    // to the shell once they finish.
    shell.run_with_input("cat >foreground", "foreground input\n\x04");

    assert_eq!(
        fs::read_to_string(shell.directory.path().join("foreground")).unwrap(),
        "foreground input\n"
    );
    shell.run_with_input(
        "cat >before-background ; sleep 0 &",
        "before background\n\x04",
    );
    assert_eq!(
        fs::read_to_string(shell.directory.path().join("before-background")).unwrap(),
        "before background\n"
    );
    shell.run_with_input(
        "sleep 0 & ; cat >after-background",
        "after background\n\x04",
    );
    assert_eq!(
        fs::read_to_string(shell.directory.path().join("after-background")).unwrap(),
        "after background\n"
    );
    shell.run_with_input(
        "cat | cat >pipeline-foreground ; sleep 0 &",
        "pipeline foreground\n\x04",
    );
    assert_eq!(
        fs::read_to_string(shell.directory.path().join("pipeline-foreground")).unwrap(),
        "pipeline foreground\n"
    );
    let output = shell.run("echo still-running");
    assert!(output.contains("\r\nstill-running\r\n"), "{output}");
}

#[test]
fn foreground_input_survives_a_later_background_list() {
    let directory = tempfile::tempdir().unwrap();
    for (command, expected) in [
        ("cat ; sleep 0 &", "payload\n"),
        ("cat | cat ; sleep 0 &", "payload\n"),
        ("cat && echo done ; sleep 0 &", "payload\ndone\n"),
        ("cat || echo unexpected ; sleep 0 &", "payload\n"),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_bsh"))
            .args([
                "--log",
                directory.path().join("shell.log").to_str().unwrap(),
                "-c",
                command,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"payload\n").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{command}");
        assert_eq!(output.stdout, expected.as_bytes(), "{command}");
    }
}
