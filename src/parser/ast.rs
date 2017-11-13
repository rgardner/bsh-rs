#[derive(Clone, Debug, PartialEq)]
pub enum Redirectee {
    Dest(i32),
    Filename(String),
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub enum Connector {
    Pipe,
    Semicolon,
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

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar;

    fn simple_command(words: Vec<&str>) -> Command {
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

    fn fd_to_file_redirection(fd: i32, filename: &str) -> Redirect {
        Redirect {
            redirector: Some(Redirectee::Dest(fd)),
            instruction: RedirectInstruction::Output,
            redirectee: Redirectee::Filename(filename.into()),
        }
    }

    #[test]
    fn test_simple_command() {
        assert!(grammar::parse_Command("").is_err());
        assert_eq!(
            grammar::parse_Command("echo bob").expect("'echo bob' should be valid"),
            simple_command(vec!["echo", "bob"])
        );
        assert_eq!(
            grammar::parse_Command("ls ~/1code").expect("'ls ~/1code' should be valid"),
            simple_command(vec!["ls", "~/1code"])
        );
        assert_eq!(
            grammar::parse_Command("echo 5").expect("'echo 5' should be valid"),
            simple_command(vec!["echo", "5"])
        );
    }

    #[test]
    fn test_input_redirection() {
        assert_eq!(
            grammar::parse_Command("echo bob <in").expect("'echo bob <in' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![input_redirection("in")],
                background: false,
            }
        );
        assert_eq!(
            grammar::parse_Command("echo bob < in").expect("'echo bob < in' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![input_redirection("in")],
                background: false,
            }
        );
        assert!(grammar::parse_Command("<").is_err());
        assert!(grammar::parse_Command("echo <").is_err());
    }

    #[test]
    fn test_output_redirection() {
        assert_eq!(
            grammar::parse_Command("echo bob >out").expect("'echo bob >out' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![output_filename_redirection("out".into())],
                background: false,
            }
        );
        assert_eq!(
            grammar::parse_Command("echo bob > out").expect("'echo bob > out' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![output_filename_redirection("out".into())],
                background: false,
            }
        );
        assert!(grammar::parse_Command(">").is_err());
        assert!(grammar::parse_Command("echo >").is_err());
    }

    #[test]
    #[ignore]
    fn test_fd_to_file_redirection() {
        assert_eq!(
            grammar::parse_Command("echo bob 1>out").expect("'echo bob 1>out' should be valid"),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![fd_to_file_redirection(1, "out".into())],
                background: false,
            }
        );
    }

    #[test]
    fn test_multiple_unique_redirection() {
        assert_eq!(
            grammar::parse_Command(">out echo <in bob").expect(
                "'>out echo <in bob' should be valid",
            ),
            Command::Simple {
                words: vec!["echo".into(), "bob".into()],
                redirects: vec![
                    output_filename_redirection("out".into()),
                    input_redirection("in".into()),
                ],
                background: false,
            }
        );
    }

    #[test]
    fn test_multiple_same_redirects() {
        assert_eq!(
            grammar::parse_Command("<in1 <in2").expect("'<in1 <in2' should be valid"),
            Command::Simple {
                words: vec![],
                redirects: vec![
                    input_redirection("in1".into()),
                    input_redirection("in2".into()),
                ],
                background: false,
            }
        );
        assert_eq!(
            grammar::parse_Command(">out1 >out2").expect("'>out1 >out2' should be valid"),
            Command::Simple {
                words: vec![],
                redirects: vec![
                    output_filename_redirection("out1".into()),
                    output_filename_redirection("out2".into()),
                ],
                background: false,
            }
        );
    }

    #[test]
    fn test_connection_command() {
        assert_eq!(
            grammar::parse_Command("cmd1 | cmd2").expect("'cmd1 | cmd2' should be valid"),
            Command::Connection {
                first: Box::new(simple_command(vec!["cmd1".into()])),
                second: Box::new(simple_command(vec!["cmd2".into()])),
                connector: Connector::Pipe,
            }
        );
        assert_eq!(
            grammar::parse_Command("cmd1 ; cmd2").expect("'cmd1 ; cmd2' should be valid"),
            Command::Connection {
                first: Box::new(simple_command(vec!["cmd1".into()])),
                second: Box::new(simple_command(vec!["cmd2".into()])),
                connector: Connector::Semicolon,
            }
        );
        assert_eq!(
            grammar::parse_Command("<in cmd1 | cmd2 >out").expect(
                "'<in cmd1 | cmd2 >out' should be valid",
            ),
            Command::Connection {
                first: Box::new(Command::Simple {
                    words: vec!["cmd1".into()],
                    redirects: vec![input_redirection("in".into())],
                    background: false,
                }),
                second: Box::new(Command::Simple {
                    words: vec!["cmd2".into()],
                    redirects: vec![output_filename_redirection("out".into())],
                    background: false,
                }),
                connector: Connector::Pipe,
            }
        );
    }

    #[test]
    fn test_long_connection_command() {
        assert_eq!(
            grammar::parse_Command("cmd1 | cmd2 | cmd3").expect(
                "'cmd1 | cmd2 | cmd3' should be valid",
            ),
            Command::Connection {
                first: Box::new(simple_command(vec!["cmd1".into()])),
                second: Box::new(Command::Connection {
                    first: Box::new(simple_command(vec!["cmd2".into()])),
                    second: Box::new(simple_command(vec!["cmd3".into()])),
                    connector: Connector::Pipe,
                }),
                connector: Connector::Pipe,
            }
        );
        assert_eq!(
            grammar::parse_Command("cmd1 | cmd2 ; cmd3").expect(
                "'cmd1 | cmd2 ; cmd3' should be valid",
            ),
            Command::Connection {
                first: Box::new(simple_command(vec!["cmd1".into()])),
                second: Box::new(Command::Connection {
                    first: Box::new(simple_command(vec!["cmd2".into()])),
                    second: Box::new(simple_command(vec!["cmd3".into()])),
                    connector: Connector::Semicolon,
                }),
                connector: Connector::Pipe,
            }
        );
    }

    #[test]
    fn test_job_background() {
        assert_eq!(
            grammar::parse_Command("cmd &").unwrap(),
            Command::Simple {
                words: vec!["cmd".into()],
                redirects: vec![],
                background: true,
            }
        );
        assert_eq!(
            grammar::parse_Command("cmd1 & | cmd2").unwrap(),
            Command::Connection {
                first: Box::new(Command::Simple {
                    words: vec!["cmd1".into()],
                    redirects: vec![],
                    background: true,
                }),
                second: Box::new(simple_command(vec!["cmd2"])),
                connector: Connector::Pipe,
            }
        );

        assert!(grammar::parse_Command("&").is_err());
    }

    #[test]
    fn test_quotes() {
        assert_eq!(
            grammar::parse_Command(">'out' 'echo' <in 'arg'").expect(
                r#">''out' 'echo' <in 'arg' should be valid"#,
            ),
            Command::Simple {
                words: vec!["echo".into(), "arg".into()],
                redirects: vec![
                    output_filename_redirection("out".into()),
                    input_redirection("in".into()),
                ],
                background: false,
            }
        );

        assert_eq!(
            grammar::parse_Command(">'out 1' echo 'arg arg arg'")
                .expect(r#"'>'out 1' echo 'arg arg arg'' should be valid"#),
            Command::Simple {
                words: vec!["echo".into(), "arg arg arg".into()],
                redirects: vec![output_filename_redirection("out 1".into())],
                background: false,
            }
        );

        assert_eq!(
            grammar::parse_Command(r#">"out" "echo" <in "arg""#)
                .expect(r#"'>"out" "echo" <in "arg"' should ve valid"#),
            Command::Simple {
                words: vec!["echo".into(), "arg".into()],
                redirects: vec![
                    output_filename_redirection("out".into()),
                    input_redirection("in".into()),
                ],
                background: false,
            }
        );

        assert!(grammar::parse_Command("echo 'arg").is_err());
        assert!(grammar::parse_Command(r#"echo "arg"#).is_err());
    }

    #[test]
    fn test_nested_quotes() {
        assert_eq!(
            grammar::parse_Command(r#"echo '"arg"'"#).expect(r#"'echo '"arg"' should be valid"#),
            simple_command(vec!["echo".into(), r#""arg""#.into()])
        );

        assert_eq!(
            grammar::parse_Command(r#"echo "'arg'""#).expect(r#"'echo "'arg'"' should be valid"#),
            simple_command(vec!["echo".into(), "'arg'".into()])
        );

        assert_eq!(
            grammar::parse_Command(r#"echo '"arg"'"#).expect(r#"'echo '"arg"'' should be valid"#),
            simple_command(vec!["echo".into(), r#""arg""#.into()])
        );

        assert_eq!(
            grammar::parse_Command(r#"echo "arg'""#).expect(r#"'echo "arg'""' should be valid"#),
            simple_command(vec!["echo".into(), r#"arg'"#.into()])
        );
    }
}
