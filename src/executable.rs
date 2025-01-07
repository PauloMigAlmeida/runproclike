use crate::cli::CliArgs;

pub struct Executable {
    pub cli_args: CliArgs,
}

impl Executable {
    pub fn new(cli_args: CliArgs) -> Self {
        Self { cli_args }
    }
}

pub trait OSSpecificExecutable {
    fn validate(pid: i32) -> anyhow::Result<()>;
    fn extract_info(&self);
}
