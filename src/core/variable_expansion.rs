use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::core::parser::ast::{visit::Visitor, Command, Connector, Redirect, Redirectee};

pub fn expand_variables<I, P, K, V>(command: &Command, home_dir: Option<P>, vars: I) -> Command
where
    P: AsRef<Path>,
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut variable_expander = VariableExpander::new(home_dir, vars);
    variable_expander.visit_command(command)
}

struct VariableExpander {
    home_dir: Option<PathBuf>,
    vars: HashMap<String, String>,
}

impl VariableExpander {
    fn new<P, I, K, V>(home_dir: Option<P>, vars: I) -> Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        Self {
            home_dir: home_dir.map(|p| p.as_ref().to_path_buf()),
            vars: vars
                .into_iter()
                .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
                .collect(),
        }
    }

    fn expand_variables_word(&self, s: &str) -> String {
        expand_variables_word(s, &self.home_dir, &self.vars)
    }
}

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
                .map(|w| self.expand_variables_word(w.as_ref()))
                .collect(),
            redirects: redirects
                .iter()
                .map(|r| Redirect {
                    redirector: match r.redirector {
                        Some(Redirectee::Filename(ref filename)) => {
                            Some(Redirectee::Filename(self.expand_variables_word(filename)))
                        }
                        ref other => other.clone(),
                    },
                    instruction: r.instruction,
                    redirectee: match r.redirectee {
                        Redirectee::Filename(ref filename) => {
                            Redirectee::Filename(self.expand_variables_word(filename))
                        }
                        ref other => other.clone(),
                    },
                })
                .collect(),
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
fn expand_variables_word<P>(s: &str, home_dir: &Option<P>, vars: &HashMap<String, String>) -> String
where
    P: AsRef<Path>,
{
    // TODO: expand tilde in any part of the word
    let expansion = match s {
        "~" => home_dir
            .as_ref()
            .map(|p| p.as_ref().to_string_lossy().into_owned()),
        s if s.starts_with('$') => vars.get(&s[1..].to_string()).cloned(),
        _ => Some(s.to_string()),
    };

    expansion.unwrap_or_else(|| "".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::iter;

    use crate::core::parser::ast::{Command, RedirectInstruction, Redirectee};

    macro_rules! generate_unique_env_key {
        () => {
            format!("KEY_LINE{}_COLUMN{}", line!(), column!())
        };
    }

    #[test]
    fn test_home_dir_expansion() {
        let command = Command::Simple {
            words: vec!["cmd1".to_string(), "~".to_string()],
            redirects: vec![Redirect {
                redirector: None,
                instruction: RedirectInstruction::Output,
                redirectee: Redirectee::Filename("~".to_string()),
            }],
            background: false,
        };

        let expected_home_dir = "MockHomeDir".to_string();
        assert_eq!(
            expand_variables(
                &command,
                Some(&expected_home_dir),
                iter::empty::<(String, String)>()
            ),
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
        let command = Command::Simple {
            words: vec!["cmd1".to_string(), format!("${}", key)],
            redirects: vec![Redirect {
                redirector: None,
                instruction: RedirectInstruction::Output,
                redirectee: Redirectee::Filename(format!("${}", key)),
            }],
            background: false,
        };

        let vars = [(key, value.clone())];
        assert_eq!(
            expand_variables(
                &command,
                None::<PathBuf>,
                vars.iter().map(|&(ref key, ref value)| (key, value))
            ),
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
