use crate::executable::Executable;
use anyhow::anyhow;
use clap::{crate_name, crate_version, ArgAction, Parser};
use procfs::process::Process;

#[derive(Parser)]
#[command(version = crate_version!(), bin_name = crate_name!())]
pub struct CliArgs {
    #[arg(short, long, help = "PID of the process of interest")]
    pub pid: i32,

    #[arg(
    long,
    help = "print the command line of the process without the path, cwd, env, etc.",
    action = ArgAction::SetTrue,
    default_value = "false"
    )]
    pub command_only: bool,

    #[arg(
    long,
    help = "Omit comments from the output.",
    action = ArgAction::SetTrue,
    default_value = "false"
    )]
    pub omit_comments: bool,
}

impl CliArgs {
    fn validate(&self) -> anyhow::Result<()> {
        // check if pid exists
        let proc = Process::new(self.pid).or(Err(anyhow!("Invalid PID {}", self.pid)))?;
        proc.status().or(Err(anyhow!(
            "read: /proc/{}/status: Permission denied",
            self.pid
        )))?;

        proc.cmdline().or(Err(anyhow!(
            "read: /proc/{}/cmdline: Permission denied",
            self.pid
        )))?;

        proc.environ().or(Err(anyhow!(
            "read: /proc/{}/environ: Permission denied",
            self.pid
        )))?;

        Ok(())
    }
}
pub fn main() {
    // parsing arguments
    let args = CliArgs::parse();
    args.validate().expect("Failed to validate arguments");

    // extract info from process
    let executable = Executable::new(args);
    executable.extract_info();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_validate_different_pid_fail() {
        let args = CliArgs {
            pid: 1,
            command_only: false,
            omit_comments: false,
        };
        let result = args.validate().unwrap_err();
        assert_eq!(
            result.to_string(),
            "read: /proc/1/environ: Permission denied"
        );
    }

    #[test]
    fn test_validate_invalid_pid_fail() {
        let args = CliArgs {
            pid: -1,
            command_only: false,
            omit_comments: false,
        };
        let result = args.validate().unwrap_err();
        assert_eq!(result.to_string(), "Invalid PID -1");
    }

    #[test]
    fn test_validate_valid_pid_success() {
        let args = CliArgs {
            pid: Process::myself().unwrap().pid() as i32,
            command_only: false,
            omit_comments: false,
        };
        let result = args.validate();
        assert!(result.is_ok());
    }
}
