use clap::{crate_name, crate_version, Parser, ArgAction};
use anyhow::anyhow;
use nix::unistd::Uid;
use procfs::process::Process;
use crate::proc::Proc;

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
}

impl CliArgs {
    fn validate(&self) -> anyhow::Result<()> {
        /*
            /proc/<pid>/cmdline (and other files) are only visible to its own process
            the only way of reading those files is to have root privileges
         */
        if !Uid::effective().is_root() {
            return Err(anyhow!("You must run this executable with root permissions"));
        }

        // check if pid exists
        Process::new(self.pid).or(Err(anyhow!("Invalid PID {}", self.pid)))?;

        Ok(())
    }
}
pub fn main() {
    // parsing arguments
    let args = CliArgs::parse();
    args.validate().expect("Failed to validate arguments");

    // extract info from process
    let executable = Proc::new(args);
    executable.extract_info();

}
