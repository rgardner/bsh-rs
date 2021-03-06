use crate::core::parser::{
    self,
    ast::{self, visit::Visitor},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Stdio {
    Inherit,
    FileDescriptor(i32),
    Filename(String),
}

impl Default for Stdio {
    fn default() -> Self {
        Stdio::Inherit
    }
}

impl From<ast::Redirect> for Stdio {
    fn from(redirect: ast::Redirect) -> Self {
        match redirect.redirectee {
            ast::Redirectee::FileDescriptor(fd) => Stdio::FileDescriptor(fd),
            ast::Redirectee::Filename(filename) => Stdio::Filename(filename),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SimpleCommand {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Stdio,
    pub stdout: Stdio,
    pub stderr: Stdio,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Simple(SimpleCommand),
    Connection {
        first: Box<Command>,
        second: Box<Command>,
        connector: ast::Connector,
    },
}

#[derive(Debug, PartialEq)]
pub struct CommandGroup {
    pub input: String,
    pub command: Command,
    pub background: bool,
}

#[derive(Debug)]
pub struct Interpreter {
    background: bool,
}

impl Visitor<Command> for Interpreter {
    fn visit_simple_command<S: AsRef<str>>(
        &mut self,
        words: &[S],
        redirects: &[ast::Redirect],
        background: bool,
    ) -> Command {
        if !self.background && background {
            self.background = background;
        }

        let (program, args) = words.split_first().unwrap();
        Command::Simple(SimpleCommand {
            program: program.as_ref().to_string(),
            args: args.iter().map(|arg| arg.as_ref().to_string()).collect(),
            stdin: get_stdin_redirect(redirects)
                .cloned()
                .map(Stdio::from)
                .unwrap_or(Stdio::Inherit),
            stdout: get_stdout_redirect(redirects)
                .cloned()
                .map(Stdio::from)
                .unwrap_or(Stdio::Inherit),
            stderr: get_stderr_redirect(redirects)
                .cloned()
                .map(Stdio::from)
                .unwrap_or(Stdio::Inherit),
        })
    }

    fn visit_connection_command(
        &mut self,
        first: &ast::Command,
        second: &ast::Command,
        connector: ast::Connector,
    ) -> Command {
        Command::Connection {
            first: Box::new(self.visit_command(first)),
            second: Box::new(self.visit_command(second)),
            connector,
        }
    }

    fn visit_command(&mut self, command: &ast::Command) -> Command {
        match command {
            ast::Command::Simple {
                ref words,
                ref redirects,
                background,
            } => self.visit_simple_command(words, redirects, *background),
            ast::Command::Connection {
                ref first,
                ref second,
                connector,
            } => self.visit_connection_command(first, second, *connector),
        }
    }
}

impl Interpreter {
    fn new() -> Interpreter {
        Interpreter { background: false }
    }

    pub fn parse(input: parser::Command) -> CommandGroup {
        let mut interpreter = Interpreter::new();
        let command = interpreter.visit_command(&input.inner);
        CommandGroup {
            input: input.input,
            command,
            background: interpreter.background,
        }
    }
}

/// Gets the last stdin redirect in `redirects`
fn get_stdin_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirect> {
    redirects.iter().rev().find(|r| is_stdin_redirect(r))
}

fn is_stdin_redirect(redirect: &ast::Redirect) -> bool {
    if (redirect.instruction != ast::RedirectInstruction::Input) || (redirect.redirector.is_some())
    {
        return false;
    }

    matches!(redirect.redirectee, ast::Redirectee::Filename(_))
}

/// Gets the last stdout redirect in `redirects`
fn get_stdout_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirect> {
    redirects.iter().rev().find(|r| is_stdout_redirect(r))
}

fn is_stdout_redirect(redirect: &ast::Redirect) -> bool {
    match redirect.redirector {
        None | Some(ast::Redirectee::FileDescriptor(1)) => (),
        _ => return false,
    }

    if redirect.instruction != ast::RedirectInstruction::Output {
        return false;
    }

    true
}

/// Gets the last stderr redirect in `redirects`
fn get_stderr_redirect(redirects: &[ast::Redirect]) -> Option<&ast::Redirect> {
    redirects.iter().rev().find(|r| is_stderr_redirect(r))
}

fn is_stderr_redirect(redirect: &ast::Redirect) -> bool {
    match redirect.redirector {
        Some(ast::Redirectee::FileDescriptor(2)) => (),
        _ => return false,
    }

    if redirect.instruction != ast::RedirectInstruction::Output {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleCommandBuilder(SimpleCommand);

    impl SimpleCommandBuilder {
        fn new(program: &str) -> Self {
            SimpleCommandBuilder(SimpleCommand {
                program: program.into(),
                args: vec![],
                stdin: Stdio::Inherit,
                stdout: Stdio::Inherit,
                stderr: Stdio::Inherit,
            })
        }

        fn arg(mut self, arg: &str) -> Self {
            self.0.args.push(arg.to_string());
            SimpleCommandBuilder(SimpleCommand {
                args: self.0.args,
                ..self.0
            })
        }

        fn stdin(self, stdin: Stdio) -> Self {
            SimpleCommandBuilder(SimpleCommand { stdin, ..self.0 })
        }

        fn stdout(self, stdout: Stdio) -> Self {
            SimpleCommandBuilder(SimpleCommand { stdout, ..self.0 })
        }

        fn stderr(self, stderr: Stdio) -> Self {
            SimpleCommandBuilder(SimpleCommand { stderr, ..self.0 })
        }

        fn build(self) -> SimpleCommand {
            self.0
        }
    }

    fn input_redirection(filename: &str) -> ast::Redirect {
        ast::Redirect {
            redirector: None,
            instruction: ast::RedirectInstruction::Input,
            redirectee: ast::Redirectee::Filename(filename.into()),
        }
    }

    fn output_filename_redirection(filename: &str) -> ast::Redirect {
        ast::Redirect {
            redirector: None,
            instruction: ast::RedirectInstruction::Output,
            redirectee: ast::Redirectee::Filename(filename.into()),
        }
    }

    fn fd_to_file_redirection(fd: i32, filename: &str) -> ast::Redirect {
        ast::Redirect {
            redirector: Some(ast::Redirectee::FileDescriptor(fd)),
            instruction: ast::RedirectInstruction::Output,
            redirectee: ast::Redirectee::Filename(filename.into()),
        }
    }

    fn fd_to_fd_redirection(
        input_fd: i32,
        instruction: ast::RedirectInstruction,
        output_fd: i32,
    ) -> ast::Redirect {
        ast::Redirect {
            redirector: Some(ast::Redirectee::FileDescriptor(input_fd)),
            instruction,
            redirectee: ast::Redirectee::FileDescriptor(output_fd),
        }
    }

    #[test]
    fn test_simple_command() {
        let input = "echo test".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "test".into()],
                    redirects: vec![],
                    background: false,
                },
            }),
            CommandGroup {
                input,
                command: Command::Simple(SimpleCommandBuilder::new("echo").arg("test").build()),
                background: false,
            }
        );
    }

    #[test]
    fn test_stdin_redirects() {
        let one_stdin_redirect_input = "echo test <in".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: one_stdin_redirect_input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "test".into()],
                    redirects: vec![input_redirection("in")],
                    background: false,
                },
            }),
            CommandGroup {
                input: one_stdin_redirect_input,
                command: Command::Simple(
                    SimpleCommandBuilder::new("echo")
                        .arg("test")
                        .stdin(Stdio::Filename("in".into()))
                        .build()
                ),
                background: false,
            }
        );

        let multiple_stdin_redirect_input = "<in1 echo test <in2".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: multiple_stdin_redirect_input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "test".into()],
                    redirects: vec![input_redirection("in1"), input_redirection("in2")],
                    background: false,
                },
            }),
            CommandGroup {
                input: multiple_stdin_redirect_input,
                command: Command::Simple(
                    SimpleCommandBuilder::new("echo")
                        .arg("test")
                        .stdin(Stdio::Filename("in2".into()))
                        .build()
                ),
                background: false,
            }
        );
    }

    #[test]
    fn test_stdout_redirects() {
        let one_stdout_redirect_input = "echo test >out".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: one_stdout_redirect_input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "test".into()],
                    redirects: vec![output_filename_redirection("out")],
                    background: false,
                },
            }),
            CommandGroup {
                input: one_stdout_redirect_input,
                command: Command::Simple(
                    SimpleCommandBuilder::new("echo")
                        .arg("test")
                        .stdout(Stdio::Filename("out".into()))
                        .build()
                ),
                background: false,
            }
        );

        let multiple_stdout_redirect_input = "<out1 echo test <out2".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: multiple_stdout_redirect_input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "test".into()],
                    redirects: vec![
                        output_filename_redirection("out1"),
                        output_filename_redirection("out2")
                    ],
                    background: false,
                },
            }),
            CommandGroup {
                input: multiple_stdout_redirect_input,
                command: Command::Simple(
                    SimpleCommandBuilder::new("echo")
                        .arg("test")
                        .stdout(Stdio::Filename("out2".into()))
                        .build()
                ),
                background: false,
            }
        );
    }

    #[test]
    fn test_stderr_redirects() {
        let one_stderr_redirect_input = "echo test 2>err".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: one_stderr_redirect_input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "test".into()],
                    redirects: vec![fd_to_file_redirection(2, "err")],
                    background: false,
                },
            }),
            CommandGroup {
                input: one_stderr_redirect_input,
                command: Command::Simple(
                    SimpleCommandBuilder::new("echo")
                        .arg("test")
                        .stderr(Stdio::Filename("err".into()))
                        .build()
                ),
                background: false,
            }
        );

        let multiple_stderr_redirect_input = "2>err1 echo test 2>err2".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: multiple_stderr_redirect_input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "test".into()],
                    redirects: vec![
                        fd_to_file_redirection(2, "err1"),
                        fd_to_file_redirection(2, "err2"),
                    ],
                    background: false,
                },
            }),
            CommandGroup {
                input: multiple_stderr_redirect_input,
                command: Command::Simple(
                    SimpleCommandBuilder::new("echo")
                        .arg("test")
                        .stderr(Stdio::Filename("err2".into()))
                        .build()
                ),
                background: false,
            }
        );
    }

    #[test]
    fn test_redirect_stderr_file() {
        let input = "2>errfile >&2 echo needle".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["echo".into(), "needle".into()],
                    redirects: vec![
                        fd_to_file_redirection(2, "errfile"),
                        fd_to_fd_redirection(1, ast::RedirectInstruction::Output, 2),
                    ],
                    background: false,
                },
            }),
            CommandGroup {
                input,
                command: Command::Simple(
                    SimpleCommandBuilder::new("echo")
                        .arg("needle")
                        .stdout(Stdio::FileDescriptor(2))
                        .stderr(Stdio::Filename("errfile".into()))
                        .build()
                ),
                background: false
            }
        )
    }

    #[test]
    fn test_connection_commands() {
        let input = "cmd1 | cmd2".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: input.clone(),
                inner: ast::Command::Connection {
                    first: Box::new(ast::Command::Simple {
                        words: vec!["cmd1".into()],
                        redirects: vec![],
                        background: false,
                    }),
                    second: Box::new(ast::Command::Simple {
                        words: vec!["cmd2".into()],
                        redirects: vec![],
                        background: false,
                    }),
                    connector: ast::Connector::Pipe,
                },
            }),
            CommandGroup {
                input,
                command: Command::Connection {
                    first: Box::new(Command::Simple(SimpleCommandBuilder::new("cmd1").build())),
                    second: Box::new(Command::Simple(SimpleCommandBuilder::new("cmd2").build())),
                    connector: ast::Connector::Pipe,
                },
                background: false,
            }
        );
    }

    #[test]
    fn test_job_background() {
        let single_ampersand_input = "cmd1 &".to_string();
        assert_eq!(
            Interpreter::parse(parser::Command {
                input: single_ampersand_input.clone(),
                inner: ast::Command::Simple {
                    words: vec!["cmd1".into()],
                    redirects: vec![],
                    background: true,
                },
            }),
            CommandGroup {
                input: single_ampersand_input,
                command: Command::Simple(SimpleCommandBuilder::new("cmd1").build()),
                background: true,
            }
        );
    }
}
