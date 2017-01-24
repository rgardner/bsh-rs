//! BSH Parser

pub use self::ast::Command;

mod ast;
#[allow(unused_qualifications, dead_code)]
mod grammar;

error_chain! {
    errors {
        /// Generic syntax error containing offending line
        SyntaxError(l: String) {
            description("syntax error")
            display("syntax error: '{}'", l)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Job {
    /// Command line, used for messages
    pub input: String,
    /// The commands to execute
    pub commands: Vec<Command>,
    /// Run the command in the background, defaults to false
    pub background: bool,
}

impl Job {
    /// Parse `input` according to bsh grammar
    pub fn parse<'input>(input: &'input str) -> Result<Option<Job>> {
        if input.is_empty() {
            Ok(None)
        } else {
            grammar::parse_Job(input)
                .map(|j| {
                    Some(Job {
                        input: input.into(),
                        commands: j.commands,
                        background: j.background,
                    })
                })
                .map_err(|_| ErrorKind::SyntaxError(input.to_string()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job_with_single_cmd(cmd: Command) -> ast::Job {
        ast::Job {
            commands: vec![cmd],
            background: false,
        }
    }

    #[test]
    fn test_simple_command() {
        assert_eq!(grammar::parse_Job("echo bob").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "bob".into()],
                       infile: None,
                       outfile: None,
                   }));
    }

    #[test]
    fn test_infile() {
        assert_eq!(grammar::parse_Job("echo bob <in").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "bob".into()],
                       infile: Some("in".into()),
                       outfile: None,
                   }));
        assert_eq!(grammar::parse_Job("echo bob < in").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "bob".into()],
                       infile: Some("in".into()),
                       outfile: None,
                   }));
        assert!(grammar::parse_Job("<").is_err());
        assert!(grammar::parse_Job("echo <").is_err());
    }

    #[test]
    fn test_outfile() {
        assert_eq!(grammar::parse_Job("echo bob >out").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "bob".into()],
                       infile: None,
                       outfile: Some("out".into()),
                   }));
        assert_eq!(grammar::parse_Job("echo bob > out").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "bob".into()],
                       infile: None,
                       outfile: Some("out".into()),
                   }));
        assert!(grammar::parse_Job(">").is_err());
        assert!(grammar::parse_Job("echo >").is_err());
    }

    #[test]
    fn test_redirect() {
        assert_eq!(grammar::parse_Job(">out echo <in bob").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "bob".into()],
                       infile: Some("in".into()),
                       outfile: Some("out".into()),
                   }));
    }

    #[test]
    fn test_multiple_redirects() {
        assert_eq!(grammar::parse_Job("<in1 <in2").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec![],
                       infile: Some("in1".into()),
                       outfile: None,
                   }));

        assert_eq!(grammar::parse_Job(">out1 >out2").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec![],
                       infile: None,
                       outfile: Some("out1".into()),
                   }));
    }

    #[test]
    fn test_job() {
        assert_eq!(grammar::parse_Job("cmd1").unwrap(),
                   ast::Job {
                       commands: vec![Command {
                                      argv: vec!["cmd1".into()],
                                      infile: None,
                                      outfile: None,
                                  }],
                       background: false,
                   });

        assert_eq!(grammar::parse_Job("<in cmd1 | cmd2 >out &").unwrap(), ast::Job {
            commands: vec![
                Command { argv: vec!["cmd1".into()], infile: Some("in".into()), outfile: None },
                Command { argv: vec!["cmd2".into()], infile: None, outfile: Some("out".into()) },
            ],
            background: true,
        });
    }

    #[test]
    fn test_job_background() {
        assert_eq!(grammar::parse_Job("cmd1 &").unwrap(),
                   ast::Job {
                       commands: vec![Command {
                                      argv: vec!["cmd1".into()],
                                      infile: None,
                                      outfile: None,
                                  }],
                       background: true,
                   });

        assert!(grammar::parse_Job("&").is_err());
        assert!(grammar::parse_Job("echo & | echo").is_err());
    }

    #[test]
    fn test_quotes() {
        assert_eq!(grammar::parse_Job(">'out 1' echo 'arg arg arg'").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "arg arg arg".into()],
                       infile: None,
                       outfile: Some("out 1".into()),
                   }));

        assert_eq!(grammar::parse_Job(">'out' 'echo' <in 'arg'").unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "arg".into()],
                       infile: Some("in".into()),
                       outfile: Some("out".into()),
                   }));

        assert_eq!(grammar::parse_Job(r#">"out" "echo" <in "arg""#).unwrap(),
                   job_with_single_cmd(Command {
                       argv: vec!["echo".into(), "arg".into()],
                       infile: Some("in".into()),
                       outfile: Some("out".into()),
                   }));
    }
}
