use crate::executable::{Executable, OSSpecificExecutable};
use libc::{c_int, c_uint, kill, pid_t, EPERM, ESRCH};
use std::cmp::Ordering;
use std::convert::TryInto;
use std::ffi::c_void;
use std::io::Error;
use std::ops::{Deref, DerefMut};
use std::{io, mem, ptr};


struct KernProcArgs2 {
    #[allow(dead_code)]
    nargs: i32,
    executable_path: String,
    #[allow(dead_code)]
    executable_cmd: String,
    executable_args: Vec<String>,
    environ: Vec<String>,
}

impl Executable {
    fn pid_exists(pid: i32) -> anyhow::Result<()> {
        // POSIX way of doing it:
        //      kill -0 <pid>
        // POSIX definition:
        //      https://pubs.opengroup.org/onlinepubs/009695399/functions/kill.html
        // Personal Rant/Suggestion:
        //      https://x.com/PauloMigAlmeida/status/1875133258717188330

        // edge case that POSIX doesn't handle (facepalm)
        if pid < 0 {
            return Err(anyhow::anyhow!("Invalid PID {pid}"));
        }

        let kill_ret = unsafe { kill(pid, 0) };

        let ret = match kill_ret.cmp(&0) {
            Ordering::Less => Error::last_os_error().raw_os_error().unwrap().into(),
            _ => 0,
        };

        match ret {
            ESRCH => Err(anyhow::anyhow!("Invalid PID {pid}")),
            EPERM => Err(anyhow::anyhow!(
                "got permission denied while checking if PID {} exits",
                pid
            )),
            _ => Ok(()),
        }
    }

    fn pid_info<T>(
        &self,
        pid: pid_t,
        flavor: c_int,
        arg: u64,
    ) -> io::Result<T> {
        let mut info = mem::MaybeUninit::<T>::uninit();
        let size = size_of::<T>() as c_int;

        let result = unsafe {
            darwin_libproc_sys::proc_pidinfo(
                pid,
                flavor,
                arg,
                info.as_mut_ptr() as *mut c_void,
                size,
            )
        };

        match result {
            value if value <= 0 => Err(Error::last_os_error()),
            value if value != size => Err(Error::new(
                io::ErrorKind::Other,
                "invalid value returned",
            )),
            _ => unsafe { Ok(info.assume_init()) },
        }
    }

    pub fn task_bsdinfo(
        &self,
        pid: pid_t,
    ) -> io::Result<darwin_libproc_sys::proc_bsdinfo> {
        self.pid_info(pid, darwin_libproc_sys::PROC_PIDTBSDINFO as c_int, 0)
    }

    fn extract_user(&self, commands: String) {
        let user = self.task_bsdinfo(self.cli_args.pid).unwrap();
        println!("sudo -i -u \\#{} <<EOF", user.pbi_uid);
        print!("{}", commands);
        println!("EOF");
    }

    fn extract_cwd(&self) -> Vec<String> {
        let mut vec = vec![];
        let cwd = darwin_libproc::pid_cwd(self.cli_args.pid as pid_t)
            .expect(format!("Couldn't retrieve cwd of pid {}", self.cli_args.pid).as_str());

        if !self.cli_args.omit_comments {
            vec.push("# change cwd user to match the target process".to_string());
        }

        vec.push(format!(
            "cd {}",
            cwd.to_str().unwrap().to_string()
        ));
        vec
    }

    fn do_sysctl(
        &self,
        mib: &mut [c_int],
        oldp: *mut c_void,
        oldlenp: *mut usize,
    ) -> Result<(), Error> {
        unsafe {
            match libc::sysctl(
                mib.as_mut_ptr() as *mut c_int,
                mib.len() as c_uint,
                oldp,
                oldlenp,
                ptr::null_mut(),
                0,
            ) {
                0 => Ok(()),
                -1 => Err(Error::last_os_error()),
                unexpected @ _ => panic!("Did not expect result code {}", unexpected),
            }
        }
    }

    fn sysctl_wrapper_default<T>(&self, mib: &mut [c_int], value: &mut T) -> Result<(), Error> {
        let mut size = size_of_val(&value);
        let pointer: *mut c_void = value as *mut _ as *mut c_void;

        self.do_sysctl(mib, pointer, &mut size as *mut usize)
    }

    fn sysctl_wrapper_heap<T: ?Sized>(
        &self,
        mib: &mut [c_int],
        value: &mut Box<T>,
    ) -> Result<(), Error> {
        let mut size = size_of_val(value.deref().deref());
        let pointer: *mut c_void = value.deref_mut().deref_mut() as *mut _ as *mut c_void;

        self.do_sysctl(mib, pointer, &mut size as *mut usize)
    }

    fn do_parse_kernel_args(
        &self,
        procargs: &Vec<u8>,
        dest: &mut String,
        mut offset: usize,
    ) -> usize {
        let mut found = false;
        let mut more = false;

        for i in offset..procargs.len() {
            let ch = procargs[i];
            if !found && ch == '\0' as u8 {
                *dest = String::from_utf8(procargs[offset..i].to_vec()).unwrap();
                found = true;
            } else if found && ch != '\0' as u8 {
                offset = i;
                more = true;
                break;
            }
        }

        if !more {
            offset = procargs.len() - 1;
        }

        offset
    }

    fn is_kernel_apple_string(&self, value: &String) -> bool {
        /*
           Kernel Apple Strings are added when execve syscall is executed
           https://github.com/apple/darwin-xnu/blob/2ff845c2e033bd0ff64b5b6aa6063a1f8f65aa32/bsd/kern/kern_exec.c#L5484-L5486

           It makes no sense to gather those as the kernel will have to generate them again.
           However, the problem is that env vars and apple strings are separated by NUL-characters
           ... and given that there is no header/indication of how many env vars are present, it's
           not possible to know where env vars stop and where apple strings start. =/
        */

        let known_apple_str = [
            "ptr_munge=",
            "main_stack=",
            "executable_file=",
            "dyld_file=",
            "executable_cdhash=",
            "executable_boothash=",
            "arm64e_abi=",
        ];

        for str in known_apple_str.iter() {
            if value.starts_with(str) {
                return true;
            }
        }
        false
    }

    fn parse_kernel_args(&self, procargs: Vec<u8>) -> KernProcArgs2 {
        let mut offset = size_of::<c_int>();

        // extract nargs
        let nargs = c_int::from_le_bytes(procargs[0..offset].try_into().unwrap());

        // extract executable path
        let mut executable_path = String::new();
        offset = self.do_parse_kernel_args(&procargs, &mut executable_path, offset);

        // extract executable cmd
        let mut executable_cmd = String::new();
        offset = self.do_parse_kernel_args(&procargs, &mut executable_cmd, offset);

        // extract executable args
        let mut executable_args = vec![];
        for _ in 1..nargs {
            let mut tmp = String::new();
            offset = self.do_parse_kernel_args(&procargs, &mut tmp, offset);
            executable_args.push(tmp);
        }

        // extract environment variables
        let mut environ = vec![];
        loop {
            let mut tmp = String::new();
            offset = self.do_parse_kernel_args(&procargs, &mut tmp, offset);
            if offset < procargs.len() - 1 {
                if !self.is_kernel_apple_string(&tmp) {
                    environ.push(tmp);
                }
            } else {
                break;
            }
        }

        KernProcArgs2 {
            nargs,
            executable_path,
            executable_cmd,
            executable_args,
            environ,
        }
    }

    fn get_kern_procargs(&self) -> KernProcArgs2 {
        // The maximum bytes of argument to execve(2).
        let mut mib = [libc::CTL_KERN, libc::KERN_ARGMAX];
        let mut len = 0;
        self.sysctl_wrapper_default(&mut mib, &mut len)
            .expect("couldn't get KERN_ARGMAX from sysctl");

        // Get cmdline and environment variables for the running PID
        let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, self.cli_args.pid];
        let mut value: Box<[u8]> = vec![0; len].into_boxed_slice();
        self.sysctl_wrapper_heap(&mut mib, &mut value)
            .expect("couldn't get KERN_PROCARGS2");
        let value = value.to_vec();

        self.parse_kernel_args(value)
    }

    fn extract_env_vars(&self, procargs: &KernProcArgs2) -> Vec<String> {
        let mut vec = vec![];

        let environ = &procargs.environ;

        if !self.cli_args.omit_comments {
            vec.push("# export env variables to match the target process".to_string());
        }

        for value in environ.iter() {
            vec.push(format!("export {value}"));
        }

        vec
    }

    fn extract_cmdline(&self, procargs: &KernProcArgs2) -> Vec<String> {
        let mut vec = vec![];
        let cmdargs = &procargs.executable_args;

        if !self.cli_args.omit_comments {
            vec.push("# cmdline to match the target process\n".to_string());
        }

        vec.push(procargs.executable_path.clone());
        for arg in cmdargs.iter() {
            vec.push(arg.clone());
        }

        vec.push("\n".to_string());
        vec
    }
}
impl OSSpecificExecutable for Executable {
    fn validate(pid: i32) -> anyhow::Result<()> {
        Self::pid_exists(pid)
    }

    fn extract_info(&self) {
        let procargs = self.get_kern_procargs();
        let cwd_lines = self.extract_cwd();
        let env_lines = self.extract_env_vars(&procargs);
        let cmd_lines = self.extract_cmdline(&procargs);

        let mut commands = vec![];
        if !self.cli_args.command_only {
            commands.push(cwd_lines.join("\n"));
            commands.push(env_lines.join("\n"));
        }
        commands.push(cmd_lines.join(" "));

        self.extract_user(commands.join("\n"));
    }
}
