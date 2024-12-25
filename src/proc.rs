use crate::cli::CliArgs;
use procfs::process::Process;

pub struct Proc {
    cli_args: CliArgs,
}

impl Proc {
    pub fn new(cli_args: CliArgs) -> Self {
        Self { cli_args }
    }

    pub fn extract_info(&self) {
        let proc_file = Process::new(self.cli_args.pid).unwrap();

        let cwd_lines = self.extract_cwd(&proc_file);
        let env_lines = self.extract_env_vars(&proc_file);
        let cmd_lines = self.extract_cmdline(&proc_file);

        let commands = vec![
            cwd_lines.join("\n"),
            env_lines.join("\n"),
            cmd_lines,
        ]
        .join("\n");

        self.extract_user(&proc_file, commands);
    }

    fn extract_user(&self, proc_file: &Process, commands: String) {
        let status = proc_file.status().unwrap();

        println!("sudo -i -u \\#{} <<EOF", status.euid);
        println!("{}", commands);
        println!("EOF");
    }

    fn extract_cwd(&self, proc_file: &Process) -> Vec<String> {
        let mut vec = vec![];
        let cwd = proc_file.cwd().unwrap();

        vec.push("# change cwd user to match the target process".to_string());
        vec.push(format!(
            "cd {}",
            cwd.into_os_string().into_string().unwrap()
        ));
        vec
    }

    fn extract_env_vars(&self, proc_file: &Process) -> Vec<String> {
        let mut vec = vec![];
        let environ = proc_file.environ().unwrap();

        vec.push("# export env variables to match the target process".to_string());

        for (key, value) in environ.into_iter() {
            vec.push(format!(
                "export {}='{}'",
                key.into_string().unwrap(),
                value.into_string().unwrap()
            ));
        }

        vec
    }

    fn extract_cmdline(&self, proc_file: &Process) -> String {
        let mut vec = vec![];
        let cmdline = proc_file.cmdline().unwrap();

        vec.push("# change cmdline to match the target process\n".to_string());

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

        vec.join("")
    }
}
