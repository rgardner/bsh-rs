use std::num::ParseIntError;
use std::result as res;

use failure::ResultExt;

use core::job::JobId;
use shell::builtins::{self, prelude::*};

pub struct Jobs;

#[derive(Debug, Deserialize)]
struct JobsArgs {
    arg_jobspec: Vec<i32>,
    flag_l: bool,
    flag_p: bool,
    flag_r: bool,
    flag_s: bool,
}

impl builtins::BuiltinCommand for Jobs {
    const NAME: &'static str = builtins::JOBS_NAME;

    const HELP: &'static str = "\
jobs: jobs [options] [<jobspec>...]

Display status of jobs.

Lists the active jobs. JOBSPEC restricts output to that job.
Without options, the status of all active jobs is displayed.alloc

Options:
    -l      lists process IDs in addition to the normal information
    -p      lists process IDs only
    -r      restrict output to running jobs
    -s      restrict output to stopped jobs

Exit Status:
Returns success unless an invalid option is given or an error occurs.";

    fn run<T: AsRef<str>>(shell: &mut Shell, args: &[T], stdout: &mut Write) -> Result<()> {
        let args: JobsArgs = parse_args(Self::HELP, Self::NAME, args.iter().map(AsRef::as_ref))?;
        debug!("{:?}", args);

        for job in &shell.get_jobs() {
            let processes = job.processes();
            if args.flag_l {
                if let Some(first) = processes.first() {
                    writeln!(
                        stdout,
                        "[{}] {:?}\t{}\t{}",
                        job.id(),
                        first.id(),
                        first.status(),
                        first.argv()
                    ).context(ErrorKind::Io)?;
                }
                for process in processes.iter().skip(1) {
                    writeln!(
                        stdout,
                        "\t{:?}\t{}\t{}",
                        process.id(),
                        process.status(),
                        process.argv()
                    ).context(ErrorKind::Io)?;
                }
            } else if args.flag_p {
                for process in processes {
                    writeln!(stdout, "{:?}", process.id()).context(ErrorKind::Io)?;
                }
            } else {
                writeln!(stdout, "{}", job).context(ErrorKind::Io)?;
            }
        }

        Ok(())
    }
}

pub struct Fg;

impl builtins::BuiltinCommand for Fg {
    const NAME: &'static str = builtins::FG_NAME;

    const HELP: &'static str = "\
fg: fg [job_spec]
    Move job to the foreground.

    Place the job identified by JOB_SPEC in the foreground, making it
    the current job. If JOB_SPEC is not present, the shell's notion of the
    current job is used.

    Exit Status:
    Status of command placed in foreground or failure if an error occurs.";

    fn run<T: AsRef<str>>(shell: &mut Shell, args: &[T], _stdout: &mut Write) -> Result<()> {
        let job_id = args
            .first()
            .map(|s| s.as_ref().parse::<u32>())
            .map_or(Ok(None), |v| v.map(Some));
        match job_id {
            Ok(job_id) => shell.put_job_in_foreground(job_id.map(JobId))?,
            Err(e) => return Err(Error::builtin_command(format!("fg: {}", e), 1)),
        };
        Ok(())
    }
}

pub struct Bg;

impl builtins::BuiltinCommand for Bg {
    const NAME: &'static str = builtins::BG_NAME;

    const HELP: &'static str = "\
bg: bg [<jobspec>...]
    Move jobs to the background.

    Place the jobs identified by each JOB_SPEC in the background, as if they
    had been started with `&'. If JOB_SPEC is not present, the shell's notion
    of the current job is used.

    Exit Status:
    Returns success unless job control is not enabled or an error occurs.";

    fn run<T: AsRef<str>>(shell: &mut Shell, args: &[T], stdout: &mut Write) -> Result<()> {
        if args.is_empty() {
            if let Err(e) = shell.put_job_in_background(None) {
                writeln!(stdout, "{}", e).context(ErrorKind::Io)?;
            }
        } else {
            let job_ids: Vec<res::Result<JobId, ParseIntError>> = args
                .iter()
                .map(|s| s.as_ref().parse::<u32>().map(JobId))
                .collect();

            for job_id in &job_ids {
                match *job_id {
                    Ok(ref job_id) => {
                        if let Err(e) = shell.put_job_in_background(Some(*job_id)) {
                            writeln!(stdout, "{}", e).context(ErrorKind::Io)?;
                        }
                    }
                    Err(ref e) => writeln!(stdout, "{}", e).context(ErrorKind::Io)?,
                }
            }
        }

        Ok(())
    }
}
