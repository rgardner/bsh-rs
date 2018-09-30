use std::{fmt, iter, process::ExitStatus};

use nix::{
    libc,
    sys::{
        signal::Signal,
        termios::{self, Termios},
    },
    unistd::Pid,
};

use util::{self, BshExitStatusExt, VecExt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcessId(u32);

impl From<u32> for ProcessId {
    fn from(value: u32) -> Self {
        ProcessId(value)
    }
}

impl From<Pid> for ProcessId {
    fn from(value: Pid) -> Self {
        libc::pid_t::from(value).into()
    }
}

impl From<libc::pid_t> for ProcessId {
    fn from(value: libc::pid_t) -> Self {
        ProcessId(value as u32)
    }
}

impl From<ProcessId> for Pid {
    fn from(value: ProcessId) -> Self {
        Pid::from_raw(value.0 as i32)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Process {
    argv: Vec<String>,
    /// `id` is None when the process hasn't launched or the command is a Shell builtin
    id: Option<ProcessId>,
    status: ProcessStatus,
    status_code: Option<ExitStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Completed,
}

impl Process {
    pub fn new_builtin<S1, S2>(program: S1, args: &[S2], status_code: ExitStatus) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Self {
            argv: iter::once(program)
                .map(|p| p.as_ref().to_string())
                .chain(args.iter().map(|arg| arg.as_ref().to_string()))
                .collect(),
            status: ProcessStatus::Completed,
            status_code: Some(status_code),
            ..Default::default()
        }
    }

    pub fn new_external<S1, S2>(program: S1, args: &[S2], id: ProcessId) -> Self
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        Self {
            argv: iter::once(&program)
                .map(|p| p.as_ref().to_string())
                .chain(args.iter().map(|arg| arg.as_ref().to_string()))
                .collect(),
            id: Some(id),
            ..Default::default()
        }
    }

    pub fn argv(&self) -> String {
        self.argv[..].join(" ")
    }

    pub fn id(&self) -> Option<ProcessId> {
        self.id
    }

    pub fn status(&self) -> ProcessStatus {
        self.status
    }

    pub fn set_status(self, status: ProcessStatus) -> Process {
        Process { status, ..self }
    }

    pub fn status_code(&self) -> Option<ExitStatus> {
        self.status_code
    }

    pub fn set_status_code(self, status_code: ExitStatus) -> Self {
        Self {
            status_code: Some(status_code),
            ..self
        }
    }

    pub fn mark_exited(self, status_code: ExitStatus) -> Self {
        Self {
            status: ProcessStatus::Completed,
            status_code: Some(status_code),
            ..self
        }
    }

    pub fn mark_stopped(self) -> Self {
        Self {
            status: ProcessStatus::Stopped,
            ..self
        }
    }
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProcessStatus::Running => write!(f, "Running"),
            ProcessStatus::Stopped => write!(f, "Stopped"),
            ProcessStatus::Completed => write!(f, "Completed"),
        }
    }
}

impl Default for ProcessStatus {
    fn default() -> Self {
        ProcessStatus::Running
    }
}

#[derive(Debug)]
pub struct ProcessGroup {
    pub id: Option<u32>,
    pub processes: Vec<Process>,
    pub foreground: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JobId(pub u32);

#[derive(Clone)]
pub struct Job {
    id: JobId,
    input: String,
    pgid: Option<libc::pid_t>,
    processes: Vec<Process>,
    last_status_code: Option<ExitStatus>,
    last_running_in_foreground: bool,
    notified_stopped_job: bool,
    tmodes: Option<Termios>,
}

impl Job {
    pub fn new(id: JobId, input: &str, pgid: Option<libc::pid_t>, processes: Vec<Process>) -> Self {
        // Initialize last_status_code if possible; this prevents a completed
        // job from having a None last_status_code if all processes have
        // already completed (e.g. 'false && echo foo')
        let last_status_code = processes
            .iter()
            .rev()
            .filter(|p| p.status_code().is_some())
            .nth(0)
            .map(|p| p.status_code().unwrap());

        Self {
            id,
            input: input.to_string(),
            pgid,
            processes,
            last_status_code,
            last_running_in_foreground: true,
            notified_stopped_job: false,
            tmodes: termios::tcgetattr(util::get_terminal()).ok(),
        }
    }

    pub fn id(&self) -> JobId {
        self.id
    }

    pub fn input(&self) -> String {
        self.input.clone()
    }

    pub fn pgid(&self) -> Option<libc::pid_t> {
        self.pgid
    }

    pub fn status(&self) -> ProcessStatus {
        if self.is_stopped() {
            ProcessStatus::Stopped
        } else if self.is_completed() {
            ProcessStatus::Completed
        } else {
            ProcessStatus::Running
        }
    }

    pub fn processes(&self) -> &Vec<Process> {
        &self.processes
    }

    pub fn last_status_code(&self) -> Option<ExitStatus> {
        self.last_status_code
    }

    pub fn last_running_in_foreground(&self) -> bool {
        self.last_running_in_foreground
    }

    pub fn set_last_running_in_foreground(self, last_running_in_foreground: bool) -> Self {
        Self {
            last_running_in_foreground,
            ..self
        }
    }

    pub fn notified_stopped_job(&self) -> bool {
        self.notified_stopped_job
    }

    pub fn set_notified_stopped_job(self, notified_stopped_job: bool) -> Self {
        Self {
            notified_stopped_job,
            ..self
        }
    }

    pub fn tmodes(&self) -> &Option<Termios> {
        &self.tmodes
    }

    pub fn is_stopped(&self) -> bool {
        self.processes
            .iter()
            .all(|p| p.status() == ProcessStatus::Stopped)
    }

    pub fn is_completed(&self) -> bool {
        self.processes
            .iter()
            .all(|p| p.status() == ProcessStatus::Completed)
    }

    pub fn mark_exited(mut self, pid: ProcessId, status_code: i32) -> Self {
        let process_index = self.find_process(pid).expect("process not found");
        let status_code = ExitStatus::from_status(status_code);
        self.processes
            .update(process_index, |p| p.mark_exited(status_code));
        Self {
            last_status_code: Some(status_code),
            ..self
        }
    }

    pub fn mark_stopped(mut self, pid: ProcessId, signal: Signal) -> Self {
        let process_index = self.find_process(pid).expect("process not found");
        self.processes.update(process_index, |p| p.mark_stopped());
        Self {
            last_status_code: Some(get_status_code_for_signal(signal)),
            ..self
        }
    }

    pub fn mark_signaled(mut self, pid: ProcessId, signal: Signal) -> Self {
        let process_index = self.find_process(pid).expect("process not found");
        let status_code = get_status_code_for_signal(signal);
        self.processes
            .update(process_index, |p| p.mark_exited(status_code));
        Self {
            last_status_code: Some(get_status_code_for_signal(signal)),
            ..self
        }
    }

    pub fn has_process(&self, pid: ProcessId) -> bool {
        self.processes
            .iter()
            .any(|p| p.id().map(|other| other == pid).unwrap_or(false))
    }

    fn find_process(&self, pid: ProcessId) -> Option<usize> {
        self.processes
            .iter()
            .position(|p| p.id().map(|other| other == pid).unwrap_or(false))
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id: {}\tinput: {}", self.id, self.input)
    }
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}\t{}", self.id, self.status(), self.input)
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn get_status_code_for_signal(signal: Signal) -> ExitStatus {
    // TODO: decide if ExitStatus should preserve signal and status
    // separately or if should combine together
    let status_code = 128 + (signal as i32);
    ExitStatus::from_status(status_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_process() {
        let process = Process::new_builtin("cmd", &["arg1"], ExitStatus::from_success());
        assert_eq!(
            process,
            Process {
                argv: vec!["cmd".to_string(), "arg1".to_string()],
                id: None,
                status: ProcessStatus::Completed,
                status_code: Some(ExitStatus::from_success())
            }
        );
    }

    #[test]
    fn test_external_process() {
        let process_id = ProcessId(1);
        let process = Process::new_external("cmd", &["arg1"], process_id);
        assert_eq!(
            process,
            Process {
                argv: vec!["cmd".to_string(), "arg1".to_string()],
                id: Some(process_id),
                status: ProcessStatus::Running,
                status_code: None,
            }
        );
    }

    #[test]
    fn test_job_is_stopped() {
        let process_id = ProcessId(1);
        let processes = vec![Process::new_external("cmd", &["arg1"], process_id)];
        let job = Job::new(JobId(1), "cmd arg1", None /*pgid*/, processes);

        assert!(!job.is_stopped());
        let job = job.mark_stopped(process_id, Signal::SIGSTOP);

        let process_index = job.find_process(process_id).expect("process not found");
        assert_eq!(job.processes[process_index].status, ProcessStatus::Stopped);
        assert!(job.is_stopped());
    }

    #[test]
    fn test_job_is_completed() {
        let process_id = ProcessId(1);
        let processes = vec![Process::new_external("cmd", &["arg1"], process_id)];
        let job = Job::new(JobId(1), "cmd arg1", None /*pgid*/, processes);

        assert!(!job.is_completed());
        let job = job.mark_exited(process_id, 0);

        let process_index = job.find_process(process_id).expect("process not found");
        assert_eq!(
            job.processes[process_index].status,
            ProcessStatus::Completed
        );
        assert_eq!(
            job.processes[process_index].status_code.unwrap(),
            ExitStatus::from_status(0)
        );
        assert!(job.is_completed());
    }
}
