#[derive(Debug, Default, PartialEq)]
pub struct Command {
    pub argv: Vec<String>,
    pub infile: Option<String>,
    pub outfile: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Job {
    pub commands: Vec<Command>,
    pub background: bool,
}
