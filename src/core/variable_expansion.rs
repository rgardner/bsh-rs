use std::env;

use dirs;

use core::parser::ast::{visit::Visitor, Command, Connector, Redirect, Redirectee};

pub fn expand_variables(command: &Command) -> Command {
    let mut variable_expander = VariableExpander;
    variable_expander.visit_command(command)
}

struct VariableExpander;

impl Visitor<Command> for VariableExpander {
    fn visit_simple_command<S: AsRef<str>>(
        &mut self,
        words: &[S],
        redirects: &[Redirect],
        background: bool,
    ) -> Command {
        Command::Simple {
            words: words
                .iter()
                .map(|w| expand_variables_word(w.as_ref()))
                .collect(),
            redirects: redirects
                .iter()
                .map(|r| Redirect {
                    redirector: match r.redirector {
                        Some(Redirectee::Filename(ref filename)) => {
                            Some(Redirectee::Filename(expand_variables_word(filename)))
                        }
                        ref other => other.clone(),
                    },
                    instruction: r.instruction,
                    redirectee: match r.redirectee {
                        Redirectee::Filename(ref filename) => {
                            Redirectee::Filename(expand_variables_word(filename))
                        }
                        ref other => other.clone(),
                    },
                }).collect(),
            background,
        }
    }

    fn visit_connection_command(
        &mut self,
        first: &Command,
        second: &Command,
        connector: Connector,
    ) -> Command {
        Command::Connection {
            first: Box::new(self.visit_command(first)),
            second: Box::new(self.visit_command(second)),
            connector,
        }
    }

    fn visit_command(&mut self, command: &Command) -> Command {
        match command {
            Command::Simple {
                ref words,
                ref redirects,
                background,
            } => self.visit_simple_command(words, redirects, *background),
            Command::Connection {
                ref first,
                ref second,
                connector,
            } => self.visit_connection_command(first, second, *connector),
        }
    }
}

/// Expands shell and environment variables in command parts.
fn expand_variables_word(s: &str) -> String {
    // TODO: extract dirs and var to function parameters
    // TODO: expand tilde in any part of the word
    let expansion = match s {
        "~" => dirs::home_dir().map(|p| p.to_string_lossy().into_owned()),
        s if s.starts_with('$') => env::var(s[1..].to_string()).ok(),
        _ => Some(s.to_string()),
    };

    expansion.unwrap_or_else(|| "".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::parser::ast::{Command, RedirectInstruction, Redirectee};

    macro_rules! generate_unique_env_key {
        () => {
            format!("KEY_LINE{}_COLUMN{}", line!(), column!())
        };
    }

    #[test]
    fn test_home_dir_expansion() {
        let expected_home_dir = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .expect("home dir not set");
        let command = Command::Simple {
            words: vec!["cmd1".to_string(), "~".to_string()],
            redirects: vec![Redirect {
                redirector: None,
                instruction: RedirectInstruction::Output,
                redirectee: Redirectee::Filename("~".to_string()),
            }],
            background: false,
        };

        assert_eq!(
            expand_variables(&command),
            Command::Simple {
                words: vec!["cmd1".to_string(), expected_home_dir.clone()],
                redirects: vec![Redirect {
                    redirector: None,
                    instruction: RedirectInstruction::Output,
                    redirectee: Redirectee::Filename(expected_home_dir)
                }],
                background: false,
            }
        );
    }

    #[test]
    fn test_env_var_expansion() {
        let key = generate_unique_env_key!();
        let value = "test".to_string();
        env::set_var(&key, &value);
        let command = Command::Simple {
            words: vec!["cmd1".to_string(), format!("${}", key)],
            redirects: vec![Redirect {
                redirector: None,
                instruction: RedirectInstruction::Output,
                redirectee: Redirectee::Filename(format!("${}", key)),
            }],
            background: false,
        };

        assert_eq!(
            expand_variables(&command),
            Command::Simple {
                words: vec!["cmd1".to_string(), value.clone()],
                redirects: vec![Redirect {
                    redirector: None,
                    instruction: RedirectInstruction::Output,
                    redirectee: Redirectee::Filename(value),
                }],
                background: false,
            }
        );
    }
}
