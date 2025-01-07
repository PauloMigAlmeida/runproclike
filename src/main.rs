mod cli;
mod executable;
#[cfg(target_os = "linux")]
mod executable_linux;
#[cfg(target_os = "macos")]
mod executable_macos;

fn main() {
    // Launch fuse client
    cli::main()
}
