use anyhow::anyhow;
use crate::cli::CliArgs;
use procfs::process::Process;
use crate::executable::{Executable, OSSpecificExecutable};

impl Executable {

    fn extract_user(&self, proc_file: &Process, commands: String) {
        let status = proc_file.status().unwrap();

        println!("sudo -i -u \\#{} <<EOF", status.euid);
        print!("{}", commands);
        println!("EOF");
    }

    fn extract_cwd(&self, proc_file: &Process) -> Vec<String> {
        let mut vec = vec![];
        let cwd = proc_file.cwd().unwrap();

        if !self.cli_args.omit_comments {
            vec.push("# change cwd user to match the target process".to_string());
        }

        vec.push(format!(
            "cd {}",
            cwd.into_os_string().into_string().unwrap()
        ));
        vec
    }

    fn extract_env_vars(&self, proc_file: &Process) -> Vec<String> {
        let mut vec = vec![];
        let environ = proc_file.environ().unwrap();

        if !self.cli_args.omit_comments {
            vec.push("# export env variables to match the target process".to_string());
        }

        for (key, value) in environ.into_iter() {
            vec.push(format!(
                "export {}='{}'",
                key.into_string().unwrap(),
                value.into_string().unwrap()
            ));
        }

        vec
    }

    fn extract_cmdline(&self, proc_file: &Process) -> Vec<String> {
        let mut vec = vec![];
        let cmdline = proc_file.cmdline().unwrap();

        if !self.cli_args.omit_comments {
            vec.push("# cmdline to match the target process\n".to_string());
        }

        for (i, arg) in cmdline.iter().enumerate() {
            let mut line = String::new();

            if i == 0 {
                line.push_str(arg);
            } else {
                line.push_str(&format!("   {}", arg));
            }

            if i < cmdline.len() - 1 {
                line.push_str(" \\");
            }

            line.push_str("\n");

            vec.push(line);
        }

        vec
    }
}

impl OSSpecificExecutable for Executable {
    fn validate(pid: i32) -> anyhow::Result<()> {
        // check if pid exists
        let proc = Process::new(pid).or(Err(anyhow!("Invalid PID {}", pid)))?;
        proc.status().or(Err(anyhow!(
            "read: /proc/{}/status: Permission denied",
            pid
        )))?;

        proc.cmdline().or(Err(anyhow!(
            "read: /proc/{}/cmdline: Permission denied",
            pid
        )))?;

        proc.environ().or(Err(anyhow!(
            "read: /proc/{}/environ: Permission denied",
            pid
        )))?;

        Ok(())
    }

    fn extract_info(&self) {
        let proc_file = Process::new(self.cli_args.pid).unwrap();

        let cwd_lines = self.extract_cwd(&proc_file);
        let env_lines = self.extract_env_vars(&proc_file);
        let cmd_lines = self.extract_cmdline(&proc_file);

        let mut commands = vec![];
        if !self.cli_args.command_only {
            commands.push(cwd_lines.join("\n"));
            commands.push(env_lines.join("\n"));
        }
        commands.push(cmd_lines.join(""));

        self.extract_user(&proc_file, commands.join("\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_cwd_helper(executable: Executable, proc_file: &Process) {
        let cwd = proc_file.cwd().unwrap();
        let mut i = 0;

        let result = executable.extract_cwd(&proc_file);

        if !executable.cli_args.omit_comments {
            assert_eq!(result[i], "# change cwd user to match the target process");
            i += 1;
        }

        assert_eq!(
            result[i],
            format!("cd {}", cwd.into_os_string().into_string().unwrap())
        );
    }
    #[test]
    fn test_extract_cwd() {
        let proc_file = Process::myself().unwrap();

        let executable_default = Executable::new(CliArgs {
            pid: proc_file.pid(),
            command_only: false,
            omit_comments: false,
        });

        extract_cwd_helper(executable_default, &proc_file);

        let executable_omit_comm = Executable::new(CliArgs {
            pid: proc_file.pid(),
            command_only: false,
            omit_comments: false,
        });

        extract_cwd_helper(executable_omit_comm, &proc_file);
    }

    fn extract_env_vars_helper(executable: Executable, proc_file: &Process) {
        let environ = proc_file.environ().unwrap();
        let result = executable.extract_env_vars(&proc_file);

        if !executable.cli_args.omit_comments {
            assert_eq!(
                result[0],
                "# export env variables to match the target process"
            );
        }

        for (key, value) in environ.into_iter() {
            let found = result.iter().find(|x| {
                x.contains(&format!(
                    "export {}='{}'",
                    key.clone().into_string().unwrap(),
                    value.clone().into_string().unwrap()
                ))
            });
            assert!(found.is_some());
        }
    }
    #[test]
    fn test_extract_env_vars() {
        let proc_file = Process::myself().unwrap();

        let executable_default = Executable::new(CliArgs {
            pid: proc_file.pid(),
            command_only: false,
            omit_comments: false,
        });

        extract_env_vars_helper(executable_default, &proc_file);

        let executable_omit_comm = Executable::new(CliArgs {
            pid: proc_file.pid(),
            command_only: false,
            omit_comments: true,
        });

        extract_env_vars_helper(executable_omit_comm, &proc_file);
    }

    fn extract_cmdline_helper(executable: Executable, proc_file: &Process) {
        let result = executable.extract_cmdline(&proc_file);
        let cmdline = proc_file.cmdline().unwrap();
        let mut i = 0;

        if !executable.cli_args.omit_comments {
            assert_eq!(result[i], "# cmdline to match the target process\n");
            i += 1;
        }

        for arg in cmdline.iter() {
            assert_eq!(
                result[i]
                    .trim_start()
                    .trim_end_matches(" \\\n")
                    .trim_end_matches("\n"),
                arg
            );
            i += 1;
        }
    }

    #[test]
    fn test_extract_cmdline() {
        let proc_file = Process::myself().unwrap();
        let executable_default = Executable::new(CliArgs {
            pid: proc_file.pid(),
            command_only: false,
            omit_comments: false,
        });

        extract_cmdline_helper(executable_default, &proc_file);
    }
}
