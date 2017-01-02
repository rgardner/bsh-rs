use errors::*;
use builtins;
use shell::Shell;

pub struct Exit;

impl builtins::BuiltinCommand for Exit {
    fn name() -> &'static str {
        builtins::EXIT_NAME
    }

    fn help() -> &'static str {
        "\
exit: exit [n]
    Exit the shell with a status of N. If N is omitted, the exit status
    is 0."
    }

    fn run(shell: &mut Shell, args: Vec<String>) -> Result<()> {
        if shell.has_background_jobs() {
            println!("There are stopped jobs.");
            return Ok(());
        }
        shell.exit(args.get(0).map(|arg| {
            arg.parse::<i32>().unwrap_or_else(|_| {
                println!("bsh: exit: {}: numeric argument required", arg);
                2
            })
        }));
        unreachable!()
    }
}
