#![allow(non_snake_case)]
#![allow(dead_code)]

use std::process::{Command, Stdio};
use std::io::Read;
use std::collections::HashMap;

pub fn run_command_capture_stdout(command: &str, args: &[&str]) -> String {
    let output = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit()) // Optionally capture or customize stderr
        .output();

    match output {
        Ok(output) if output.status.success() => {
            // Command succeeded, return stdout as a String
            String::from_utf8_lossy(&output.stdout).to_string()
        }
        _ => {
            // Command failed or an error occurred
            eprintln!("Failed to execute the command");
            String::new() // Return an empty string
        }
    }
}

#[derive(Debug)]
pub struct ProcessInfo {
    pub ppid: i32,
    pub tty: String,
}

pub fn parse_ps_output(input: &str) -> HashMap<i32, ProcessInfo> {
    let mut process_map = HashMap::new();

    for line in input.lines() {
        let mut columns = line.split_whitespace();

        if let (Some(pid), Some(ppid), Some(tty)) = (
            columns.next().map(|s| s.parse::<i32>().ok()).flatten(),
            columns.next().map(|s| s.parse::<i32>().ok()).flatten(),
            columns.next(),
        ) {
            let tty_with_dev = format!("/dev/{}", tty);
            process_map.insert(
                pid,
                ProcessInfo {
                    ppid,
                    tty: tty_with_dev,
                },
            );
        }
    }

    process_map
}

/////////////////////////////////////////////////////////
// NOTE: this CAN be done in a much more clever way... //
/////////////////////////////////////////////////////////
pub fn insert_in_buffer( buffer: &mut [u8], bufferLen: usize, bufferPos: &mut usize, insertBytes: &[u8], numBytes: usize ) {
    for insertPos in 0..numBytes {
        let insertChar = insertBytes[insertPos];

        if *bufferPos==bufferLen {
            // at end of buffer: make space for one more character
            for i in 1..bufferLen {
                buffer[i-1] = buffer[i];
            }
            *bufferPos = *bufferPos - 1;
        }

        buffer[*bufferPos] = insertChar;
        *bufferPos = *bufferPos + 1;
    }
}

/////////////////////////////////////////////////////////
// NOTE: this was tested on FreeBSD only...            //
/////////////////////////////////////////////////////////
pub fn get_child_TTY( ppid: i32 ) -> String {
    let command             = "ps";
    let args                = &["-o", "tty", "-p", &ppid.to_string()];
    let output              = run_command_capture_stdout(command, args);
    let mut capture         = false;
    let mut devTTY : String = String::new();

    for line in output.lines() {
        if capture {
            devTTY = format!("/dev/{}", line);
        } else {
            capture = true;
        }
    }
    return devTTY; // e.g. of return is "/dev/pts/1"
}

pub fn ensure_terminates_with_crlf(s: String) -> String {
    let mut sCopy = s.clone();
    if !sCopy.ends_with("\r\n") {
        if s.ends_with("\n\r") {
            sCopy = sCopy.replace("\n\r", "\r\n");
        } else if sCopy.ends_with("\r") {
            sCopy = sCopy.replace("\r","\r\n");
        } else if sCopy.ends_with("\n") {
            sCopy = sCopy.replace("\n", "\r\n");
        }
    }
    return sCopy;
}

