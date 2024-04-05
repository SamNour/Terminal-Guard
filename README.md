![pptCC69 pptm  -  AutoRecovered](https://github.com/SamNour/Terminal-Guard/assets/96638051/62c5289a-6e46-477b-9505-e1a320ceae2e)


# Terminal-Guard
Terminal-Guard is a minimalistic Pseudo-Terminal (PTY) that controls interactive programs. It gives the user complete control over the interactions between the Parent-Child processes. The PTY is totally controlled by a -YAML configuration file written in lua.

The tool is aimed at security and Linux education; applications like https://overthewire.org/wargames/ inspire it, but it gives the developer much more flexibility due to its design.

*Disclaimer: the source code provided does not work as intended; updated branches will be available in August 2024 for copyright requirements; for more information, contact sam.nour@tum.de*.


### Table of contents  
- [Requirements](#requirements)
 - [Help](#help) 


### Requirements
- Curl
- Cargo:
	-  To get the latest release, install the current stable release of Rust
	- ``curl https://sh.rustup.rs -sSf | sh``


## Help
```$ cargo run -- --help
Man-in-the-Middle Terminal Multiplexer (MinMux) 

USAGE:
    minmux --config <FILE> --exec <EXEC> --prompt <PROMPT>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <FILE>      The YAML file to read [default: setup.yaml]
    -e, --exec <EXEC>        The executable to run in the terminal [default: /bin/bash]
    -p, --prompt <PROMPT>    The prompt to look for [default: \[\w+@.+\s+.*\]\$\s+]

```

## To run
example
``` cargo run -- --config ../setup.yaml --exec /bin/bash --prompt "\\[\\w+@.+\\s+.*\\]\\$\\s+"]```
