#[derive(Clone, Debug, PartialEq)]
pub enum Redirectee {
    FileDescriptor(i32),
    Filename(String),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RedirectInstruction {
    Output,
    Input,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Redirect {
    pub redirector: Option<Redirectee>,
    pub instruction: RedirectInstruction,
    pub redirectee: Redirectee,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Connector {
    Pipe,
    Semicolon,
    And,
    Or,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Simple {
        words: Vec<String>,
        redirects: Vec<Redirect>,
        background: bool,
    },
    Connection {
        first: Box<Command>,
        second: Box<Command>,
        connector: Connector,
    },
}

#[derive(Debug, Default)]
pub struct SimpleCommandBuilder {
    pub words: Vec<String>,
    pub redirects: Vec<Redirect>,
    pub background: bool,
}

impl SimpleCommandBuilder {
    pub fn new(background: bool) -> SimpleCommandBuilder {
        SimpleCommandBuilder {
            background,
            ..Default::default()
        }
    }

    pub fn update(mut self, command_part: SimpleCommandPart) -> SimpleCommandBuilder {
        match command_part {
            SimpleCommandPart::Word(w) => self.words.push(w),
            SimpleCommandPart::Redirect(r) => self.redirects.push(r),
        };

        self
    }

    pub fn build(&self) -> Command {
        Command::Simple {
            words: self.words.clone(),
            redirects: self.redirects.clone(),
            background: self.background,
        }
    }
}

#[derive(Debug)]
pub enum SimpleCommandPart {
    Word(String),
    Redirect(Redirect),
}

pub mod visit {
    use super::*;

    pub trait Visitor<T> {
        fn visit_simple_command<S: AsRef<str>>(
            &mut self,
            words: &[S],
            redirects: &[Redirect],
            background: bool,
        ) -> T;

        fn visit_connection_command(
            &mut self,
            first: &Command,
            second: &Command,
            connector: Connector,
        ) -> T;

        fn visit_command(&mut self, command: &Command) -> T;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parser::grammar::CommandParser;

    fn simple_command(words: &[&str]) -> Command {
        Command::Simple {
            words: words.iter().map(|s| s.to_string()).collect(),
            redirects: vec![],
            background: false,
        }
    }

    fn input_redirection(filename: &str) -> Redirect {
        Redirect {
            redirector: None,
            instruction: RedirectInstruction::Input,
            redirectee: Redirectee::Filename(filename.into()),
        }
    }

    fn output_filename_redirection(filename: &str) -> Redirect {
        Redirect {
            redirector: None,
            instruction: RedirectInstruction::Output,
            redirectee: Redirectee::Filename(filename.into()),
        }
    }

    fn output_fd_redirection(fd: i32) -> Redirect {
        Redirect {
            redirector: None,
            instruction: RedirectInstruction::Output,
            redirectee: Redirectee::FileDescriptor(fd),
        }
    }

    fn fd_to_file_redirection(fd: i32, filename: &str) -> Redirect {
        Redirect {
            redirector: Some(Redirectee::FileDescriptor(fd)),
            instruction: RedirectInstruction::Output,
            redirectee: Redirectee::Filename(filename.into()),
        }
    }

    fn fd_to_fd_redirection(
        input_fd: i32,
        instruction: RedirectInstruction,
        output_fd: i32,
    ) -> Redirect {
        Redirect {
            redirector: Some(Redirectee::FileDescriptor(input_fd)),
            instruction,
            redirectee: Redirectee::FileDescriptor(output_fd),
        }
    }

    #[test]
    fn test_simple_command() {
        assert!(CommandParser::new().parse("").is_err());
        assert_eq!(
            CommandParser::new()
                .parse("echo bob")
                .expect("'echo bob' should be valid"),
            simple_command(&["echo", "bob"])
        );
        assert_eq!(
            CommandParser::new()
                .parse("ls ~/1code")
                .expect("'ls ~/1code' should be valid"),
            simple_command(&["ls", "~/1code"])
        );
        assert_eq!(
            CommandParser::new()
                .parse("echo 5")
                .expect("'echo 5' should be valid"),
            simple_command(&["echo", "5"])
        );
    }

    #[test]
    fn test_input_redirection() {
        assert_eq!(
            CommandParser::new()
                .parse("echo bob <in")
                .expect("'echo bob <in' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![input_redirection("in")],
                background: false,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("echo bob < in")
                .expect("'echo bob < in' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![input_redirection("in")],
                background: false,
            }
        );
        assert!(CommandParser::new().parse("<").is_err());
        assert!(CommandParser::new().parse("echo <").is_err());
    }

    #[test]
    fn test_output_redirection() {
        assert_eq!(
            CommandParser::new()
                .parse("echo bob >out")
                .expect("'echo bob >out' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![output_filename_redirection("out")],
                background: false,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("echo bob > out")
                .expect("'echo bob > out' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![output_filename_redirection("out")],
                background: false,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("echo bob 1>out")
                .expect("'echo bob 1>out' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![fd_to_file_redirection(1, "out")],
                background: false,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("echo bob 1> out")
                .expect("'echo bob 1>out' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![fd_to_file_redirection(1, "out")],
                background: false,
            }
        );
        assert!(CommandParser::new().parse(">").is_err());
        assert!(CommandParser::new().parse("echo >").is_err());
    }

    #[test]
    fn test_fd_duplication() {
        assert_eq!(
            CommandParser::new()
                .parse("echo bob 1>&2")
                .expect("'echo bob 1>&2' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![fd_to_fd_redirection(1, RedirectInstruction::Output, 2)],
                background: false,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("echo bob 2<&1")
                .expect("'echo bob 2>&1' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![fd_to_fd_redirection(2, RedirectInstruction::Input, 1)],
                background: false,
            }
        );
    }

    #[test]
    fn test_multiple_unique_redirection() {
        assert_eq!(
            CommandParser::new()
                .parse(">out echo <in bob",)
                .expect("'>out echo <in bob' should be valid",),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![output_filename_redirection("out"), input_redirection("in"),],
                background: false,
            }
        );

        assert_eq!(
            CommandParser::new()
                .parse("2>errfile >&2 echo needle",)
                .expect("'2>errfile >&2 echo needle' should be valid",),
            Command::Simple {
                words: vec!["echo".into(), "needle".into()],
                redirects: vec![
                    fd_to_file_redirection(2, "errfile"),
                    output_fd_redirection(2),
                ],
                background: false,
            }
        );
    }

    #[test]
    fn test_multiple_same_redirects() {
        assert_eq!(
            CommandParser::new()
                .parse("<in1 <in2")
                .expect("'<in1 <in2' should be valid"),
            Command::Simple {
                words: vec![],
                redirects: vec![input_redirection("in1"), input_redirection("in2"),],
                background: false,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse(">out1 >out2")
                .expect("'>out1 >out2' should be valid"),
            Command::Simple {
                words: vec![],
                redirects: vec![
                    output_filename_redirection("out1"),
                    output_filename_redirection("out2"),
                ],
                background: false,
            }
        );
    }

    #[test]
    fn test_connection_command() {
        assert_eq!(
            CommandParser::new()
                .parse("cmd1 | cmd2")
                .expect("'cmd1 | cmd2' should be valid"),
            Command::Connection {
                first: Box::new(simple_command(&["cmd1"])),
                second: Box::new(simple_command(&["cmd2"])),
                connector: Connector::Pipe,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("cmd1 ; cmd2")
                .expect("'cmd1 ; cmd2' should be valid"),
            Command::Connection {
                first: Box::new(simple_command(&["cmd1"])),
                second: Box::new(simple_command(&["cmd2"])),
                connector: Connector::Semicolon,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("<in cmd1 | cmd2 >out",)
                .expect("'<in cmd1 | cmd2 >out' should be valid",),
            Command::Connection {
                first: Box::new(Command::Simple {
                    words: vec!["cmd1".into()],
                    redirects: vec![input_redirection("in")],
                    background: false,
                }),
                second: Box::new(Command::Simple {
                    words: vec!["cmd2".into()],
                    redirects: vec![output_filename_redirection("out")],
                    background: false,
                }),
                connector: Connector::Pipe,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("cmd1 && cmd2")
                .expect("'cmd1 && cmd2' should be valid"),
            Command::Connection {
                first: Box::new(Command::Simple {
                    words: vec!["cmd1".into()],
                    redirects: vec![],
                    background: false,
                }),
                second: Box::new(Command::Simple {
                    words: vec!["cmd2".into()],
                    redirects: vec![],
                    background: false,
                }),
                connector: Connector::And
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("cmd1 || cmd2")
                .expect("'cmd1 || cmd2' should be valid"),
            Command::Connection {
                first: Box::new(Command::Simple {
                    words: vec!["cmd1".into()],
                    redirects: vec![],
                    background: false,
                }),
                second: Box::new(Command::Simple {
                    words: vec!["cmd2".into()],
                    redirects: vec![],
                    background: false,
                }),
                connector: Connector::Or
            }
        );
    }

    #[test]
    fn test_long_connection_command() {
        assert_eq!(
            CommandParser::new()
                .parse("cmd1 | cmd2 | cmd3",)
                .expect("'cmd1 | cmd2 | cmd3' should be valid",),
            Command::Connection {
                first: Box::new(simple_command(&["cmd1"])),
                second: Box::new(Command::Connection {
                    first: Box::new(simple_command(&["cmd2"])),
                    second: Box::new(simple_command(&["cmd3"])),
                    connector: Connector::Pipe,
                }),
                connector: Connector::Pipe,
            }
        );
        assert_eq!(
            CommandParser::new()
                .parse("cmd1 | cmd2 ; cmd3",)
                .expect("'cmd1 | cmd2 ; cmd3' should be valid",),
            Command::Connection {
                first: Box::new(simple_command(&["cmd1"])),
                second: Box::new(Command::Connection {
                    first: Box::new(simple_command(&["cmd2"])),
                    second: Box::new(simple_command(&["cmd3"])),
                    connector: Connector::Semicolon,
                }),
                connector: Connector::Pipe,
            }
        );
    }

    #[test]
    fn test_job_background() {
        assert_eq!(
            CommandParser::new().parse("cmd &").unwrap(),
            Command::Simple {
                words: vec!["cmd".into()],
                redirects: vec![],
                background: true,
            }
        );
        assert_eq!(
            CommandParser::new().parse("cmd1 & | cmd2").unwrap(),
            Command::Connection {
                first: Box::new(Command::Simple {
                    words: vec!["cmd1".into()],
                    redirects: vec![],
                    background: true,
                }),
                second: Box::new(simple_command(&["cmd2"])),
                connector: Connector::Pipe,
            }
        );

        assert!(CommandParser::new().parse("&").is_err());
    }

    #[test]
    fn test_quotes() {
        assert_eq!(
            CommandParser::new()
                .parse(">'out' 'echo' <in 'arg'",)
                .expect(r#">''out' 'echo' <in 'arg' should be valid"#,),
            Command::Simple {
                words: vec!["echo".into(), "arg".into()],
                redirects: vec![output_filename_redirection("out"), input_redirection("in"),],
                background: false,
            }
        );

        assert_eq!(
            CommandParser::new()
                .parse(">'out 1' echo 'arg arg arg'")
                .expect(r#"'>'out 1' echo 'arg arg arg'' should be valid"#),
            Command::Simple {
                words: vec!["echo".into(), "arg arg arg".into()],
                redirects: vec![output_filename_redirection("out 1")],
                background: false,
            }
        );

        assert_eq!(
            CommandParser::new()
                .parse(r#">"out" "echo" <in "arg""#)
                .expect(r#"'>"out" "echo" <in "arg"' should ve valid"#),
            Command::Simple {
                words: vec!["echo".into(), "arg".into()],
                redirects: vec![output_filename_redirection("out"), input_redirection("in"),],
                background: false,
            }
        );

        assert!(CommandParser::new().parse("echo 'arg").is_err());
        assert!(CommandParser::new().parse(r#"echo "arg"#).is_err());
    }

    #[test]
    fn test_nested_quotes() {
        assert_eq!(
            CommandParser::new()
                .parse(r#"echo '"arg"'"#)
                .expect(r#"'echo '"arg"' should be valid"#),
            simple_command(&["echo", r#""arg""#])
        );

        assert_eq!(
            CommandParser::new()
                .parse(r#"echo "'arg'""#)
                .expect(r#"'echo "'arg'"' should be valid"#),
            simple_command(&["echo", "'arg'"])
        );

        assert_eq!(
            CommandParser::new()
                .parse(r#"echo '"arg"'"#)
                .expect(r#"'echo '"arg"'' should be valid"#),
            simple_command(&["echo", r#""arg""#])
        );

        assert_eq!(
            CommandParser::new()
                .parse(r#"echo "arg'""#)
                .expect(r#"'echo "arg'""' should be valid"#),
            simple_command(&["echo", r#"arg'"#])
        );
    }

    #[test]
    fn test_quotes_allow_special_characters() {
        assert_eq!(
            CommandParser::new()
                .parse(r#"echo '& ; echo |'"#,)
                .expect(r#"'echo '& ; echo |'' should be valid"#,),
            simple_command(&["echo", r#"& ; echo |"#])
        );
    }
}
