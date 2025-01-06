# runproclike
[![Build](https://github.com/PauloMigAlmeida/runproclike/actions/workflows/rust.yml/badge.svg)](https://github.com/PauloMigAlmeida/runproclike/actions/workflows/rust.yml)
[![Release](https://github.com/PauloMigAlmeida/runproclike/actions/workflows/release.yml/badge.svg)](https://github.com/PauloMigAlmeida/runproclike/actions/workflows/release.yml)
[![Crate](https://img.shields.io/crates/v/runproclike.svg)](https://crates.io/crates/runproclike)

`runproclike` is a lightweight command-line utility that analyzes a running process by its PID and prints the command and its environment in a reproducible form. It's especially useful for debugging, replicating process states, or migrating workloads.

## Features

- Generates a detailed output of the process's environment, command line, and execution context.
- Inspired by the [runlike](https://github.com/lavie/runlike) tool, which focuses on reconstructing Docker commands. `runproclike` extends this concept to Linux processes.

## Installation

### Using Cargo

```bash
export PATH=~/.cargo/bin/:$PATH
cargo install runproclike
```

### Using Docker
Don't want to build and install anything? All good, you can leverage the docker image
built precisely for that!

```bash
docker run \
  --rm \
  --privileged \
  --pid=host \
  ghcr.io/paulomigalmeida/runproclike:latest \
  --pid <pid>
```

### Building from source
1. Clone the repository:
   ```bash
   git clone https://github.com/PauloMigAlmeida/runproclike.git
   cd runproclike
   ```
2. Build
   ```bash
   cargo build --release
   ```

## Usage

`runproclike [OPTIONS] --pid <PID>`

### Options:

```bash
Usage: runproclike [OPTIONS] --pid <PID>

Options:
  -p, --pid <PID>      PID of the process of interest
      --command-only   print the command line of the process without the path, cwd, env, etc.
      --omit-comments  Omit comments from the output.
  -h, --help           Print help
  -V, --version        Print version
```

### Examples

Reproduce a Process:

```bash
runproclike --pid 19352

sudo -i -u \#1000 <<EOF
  # change cwd user to match the target process
  cd /home/paulo/workspace/runproclike
  
  # export env variables to match the target process
  export WAYLAND_DISPLAY='wayland-0'
  export USER='paulo'
  export XDG_MENU_PREFIX='gnome-'
  export LANG='en_NZ.UTF-8'
  
  # cmdline to match the target process
  /bin/bash \
     --rcfile \
    /home/paulo/IDE/RustRover-2024.3.2/plugins/terminal/shell-integrations/bash/bash-integration.bash \
    -i
EOF
```

## How it Works

`runproclike` inspects a process's details by reading procfs files and outputs the information needed to recreate the process in a shell.

## Contributing

Contributions are welcome! Please follow these steps:

- Fork the repository.
- Create a new branch for your feature/fix.
- Write tests ;)
- Submit a pull request.
