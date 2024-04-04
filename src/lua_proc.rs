
#![allow(unused_unsafe)]

use mlua::{Lua, Function, Result};
use lazy_static::lazy_static;
use std::sync::RwLock;
use libc;
use libc::c_void;
use libc::size_t;
use libc::fsync;
use libc::write;
use std::process::exit;
use regex::Regex;
use crate::pty_process;
use crate::utilities;
use std::collections::HashMap;

pub fn luaSendKeys( lua: &Lua, keys: String, untilStr: String ) -> Result<String> {
    unsafe {
        let ptr = keys.as_ptr() as *const c_void;
        let len = keys.len();
        let getFD = lua.globals().get("LUA_GLOBAL_FD");
        let fd = match getFD { Ok(f) => {f}, Err(_) => {0} };

        libc::write(fd, ptr, len);

        fsync(fd);
        // read back the echo from the terminal...
        let otherKeys = utilities::ensure_terminates_with_crlf(keys);
        readUntil(fd,otherKeys);

        // read until the untilStr is a "hit"
        let r = readUntil(fd,untilStr);
        Ok(r)
    }
}

pub fn luaSendKeys2( lua: &Lua, keys: String ) -> Result<()> {
    unsafe {
        let ptr = keys.as_ptr() as *const c_void;
        let len = keys.len();
        let getFD = lua.globals().get("LUA_GLOBAL_FD");
        let fd = match getFD { Ok(f) => {f}, Err(_) => {0} };

        libc::write(fd, ptr, len);

        fsync(fd);
        Ok(())
    }
}

fn readUntil(fd: i32, rx : String) -> String {
    const captureBufferLen : usize = 1024;
    let mut captureBuffer: [u8; captureBufferLen] = [0; captureBufferLen];
    let mut captureBufferPos = 0;

    loop {
        let mut read_buf: [u8; 256] = [0; 256];
        let num_read = pty_process::fd_read(fd, &mut read_buf, 256) as i64;
        if num_read>0 {
            utilities::insert_in_buffer( &mut captureBuffer,
                                         captureBufferLen,
                                         &mut captureBufferPos,
                                         &read_buf,
                                         num_read as size_t );
            let theBuffer = String::from_utf8_lossy(&captureBuffer[..captureBufferPos]);
            let re = Regex::new(rx.as_str()).unwrap();
            //print!("TEST '{}' ON '{}'\r\n",rx,theBuffer);
            if re.is_match(&theBuffer) { break; }
        } else {
            // this should NEVER happen!
            exit(0);
        }
    }
    let theCBuffer = String::from_utf8_lossy(&captureBuffer[..captureBufferPos]);
    return theCBuffer.to_string();
}

pub fn luaBackSpace( lua: &Lua ) -> Result<()> {
    unsafe {
        let getFD = lua.globals().get("LUA_GLOBAL_FD");
        let fd = match getFD { Ok(f) => {f}, Err(_) => {0} };
        let getNK = lua.globals().get("LUA_GLOBAL_NK");
        let nk = match getNK { Ok(n) => {n}, Err(_) => {0} };
        for i in 0..nk {
            let write_buf: [u8; 1] = [127; 1];  // prefer backspace (127) instead of delete (8)
            let ptr = write_buf.as_ptr() as *const c_void;
            libc::write(fd, ptr, 1);
        }
        Ok(())
    }
}

pub fn luaPrgRunning( lua: &Lua ) -> Result<i32> {
    let mut PIDmap : Vec<(i32, i32)> = Vec::new();
    let psOut = utilities::run_command_capture_stdout("ps",&["-o","pid,ppid"]);

    for line in psOut.lines() {
        let mut columns = line.split_whitespace();
        if let (Some(pid), Some(ppid)) = (
            columns.next().map(|s| s.parse::<i32>().ok()).flatten(),
            columns.next().map(|s| s.parse::<i32>().ok()).flatten(),
        ) {
            PIDmap.push( (pid, ppid) );
        }
    }

    let mut startVec : Vec<(i32, i32)> = Vec::new();
    let getPID = lua.globals().get("LUA_GLOBAL_PID");
    let pid = match getPID { Ok(p) => {p}, Err(_) => {0} };
    startVec.push( (pid,0) );

    loop {
        let mut flagDone = true;
        let mut newVec : Vec<(i32, i32)> = Vec::new();

        for (pid,op) in startVec.iter() {
            newVec.push( (*pid,-1) );
            if 0==*op {
                for (pid2,ppid2) in PIDmap.iter() {
                    if *ppid2==*pid {
                        newVec.push( (*pid2,0) );
                        flagDone = false;
                    }
                }
            }
        }

        startVec = newVec.clone();
        if flagDone { break; }
    }
    Ok(startVec.len() as i32)
}

