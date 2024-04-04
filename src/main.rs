
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

mod pty_process;
use regex::Regex;
mod select_process;
mod utilities;
mod config;
mod lua_proc;
use std::collections::HashMap;

use libc::{c_void, cfmakeraw, setsid, size_t, TCSANOW, TIOCGWINSZ, TIOCSCTTY, TIOCSWINSZ};
use nix::libc::{ioctl, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
use nix::unistd::{close, fork, ForkResult};

use std::os::unix::io::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};
use std::time;
extern crate libc;
use clap::Arg;
use clap::App;
use mlua::Lua;


fn normal_exit() -> () {
    print!("\r\nExiting min-mux\r\n");
    exit(0);
}

fn main() -> () {
    // normal keyboard buffer
    const kbdBufferLen : usize = 1024;
    let mut kbdBuffer: [u8; kbdBufferLen] = [0; kbdBufferLen];
    let mut kbdBufferPos = 0;
    //let lua = LuaProc::new();

    let lua = mlua::Lua::new();

    // Create a Lua context and set the function as a global variable
    let luaGlobals = lua.globals();

    let luaFunction = lua.create_function(|lua,(arg1,arg2):(String,String)|{lua_proc::luaSendKeys(lua,arg1,arg2)});
    match luaFunction {
        Ok(func) => { let _ = luaGlobals.set("sendKeys", func);              }
        _ =>        { println!("ERROR: could not create function"); exit(0); }
    }
    let luaFunction = lua.create_function(lua_proc::luaSendKeys2);
    match luaFunction {
        Ok(func) => { let _ = luaGlobals.set("sendKeys2", func);             }
        _ =>        { println!("ERROR: could not create function"); exit(0); }
    }
    let luaFunction = lua.create_function(|lua, _: ()| { lua_proc::luaBackSpace(lua) });
    match luaFunction {
        Ok(func) => { let _ = luaGlobals.set("backSpace", func);             }
        _ =>        { println!("ERROR: could not create function"); exit(0); }
    }
    let luaFunction = lua.create_function(|lua, _: ()| { lua_proc::luaPrgRunning(lua) });

    match luaFunction {
        Ok(func) => { let _ = luaGlobals.set("prgRunning", func);            }
        _ =>        { println!("ERROR: could not create function"); exit(0); }
    }

    let bashPath = if      cfg!(target_os = "freebsd") { "/usr/local/bin/bash" }
    else if cfg!(target_os = "linux")   { "/bin/bash"           }
    else                                { "UNKNOWN"             };
    if bashPath=="UNKNOWN" {
        println!("Unknown OS type!");
        normal_exit();
    }

    let matches = App::new("Man-in-the-Middle Terminal Multiplexer (MinMux)")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("The YAML file to read")
            .takes_value(true)
            .required(true)
            .default_value("setup.yaml")
        )
        .arg(Arg::with_name("exec")
            .short("e")
            .long("exec")
            .value_name("EXEC")
            .help("The executable to run in the terminal")
            .takes_value(true)
            .required(true)
            .default_value(bashPath)
        )
        .arg(Arg::with_name("prompt")
            .short("p")
            .long("prompt")
            .value_name("PROMPT")
            .help("The prompt to look for")
            .takes_value(true)
            .required(true)
            .default_value("\\[\\w+@.+\\s+.*\\]\\$\\s+")
    )
        .get_matches();

    let configYAML = matches.value_of("config").unwrap();
    let configDict : HashMap<String, config::ConfigEntry>;
    let execProg = matches.value_of("exec").unwrap();
    let prompt = matches.value_of("prompt").unwrap();

    match config::parse_yaml_to_dict(configYAML) {
        Ok(config_dict) => {
            configDict = config_dict.clone();
            for (id, entry) in config_dict.iter() {
                if entry.when.to_uppercase()=="LUA" {
                    let _ = lua.load(&entry.code.to_string()).exec();
                }
            }
        }
        Err(err) => {
            eprintln!("Error reading YAML file: {}", err);
            exit(0);
        }
    }
    let _ = luaGlobals.set("PROMPT_RE",prompt.to_string());

    unsafe {
        libc::tcgetattr(STDIN_FILENO, &mut pty_process::SAVE_TERM);
        libc::atexit(pty_process::restore_term);
    };

    if pty_process::install_signal_handler() == false {
        println!("ERROR: could not install signal handler!");
        std::process::exit(1)
    }

    // get window dimensions
    let w = pty_process::Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &w) };

    // Open a master pty
    let pty = pty_process::new();
    match unsafe { fork().expect("Failed during fork()") } {
        ForkResult::Parent { child, .. } => {
            let _ = luaGlobals.set("LUA_GLOBAL_PID",child.as_raw());
            // This is the parent process
            unsafe {
                let mut new_term: libc::termios = pty_process::SAVE_TERM.clone();
                cfmakeraw(&mut new_term);
                libc::tcsetattr(STDIN_FILENO, TCSANOW, &new_term);
            }
            // Close the slave pty file descriptor
            close(pty.fds).expect("Error during closing the pty.fds in the child match arm");
            
            const captureScreenBufferLen : usize = 1024;
            let mut captureScreenBuffer: [u8; captureScreenBufferLen] = [0; captureScreenBufferLen];
            let mut captureScreenBufferPos = 0;

            loop {
                unsafe {
                    let mut fd_set = select_process::FdSet::new();
                    fd_set.set(0);                        // we want to observe the keyboard
                    fd_set.set(pty.fdm.as_raw_fd());      // we want to observe output from the PTY
                    let _ = luaGlobals.set("LUA_GLOBAL_FD",pty.fdm.as_raw_fd()); // tell Lua which file descriptor to look at

                    let max_fd = pty.fdm.as_raw_fd();
                    match select_process::select( max_fd + 1,
                        Some(&mut fd_set),                                // read
                        None,                                             // write
                        None,                                             // error
                        Some(&select_process::make_timeval(time::Duration::new(60,0))), ) // timeout (sec, nsec)
                    {
                        Ok(_) => {
                            let range = std::ops::Range { start: 0, end: max_fd + 1, };
                            for i in range {
                                if (fd_set).is_set(i) {
                                    if 0==i {  // read from the keyboard
                                        let mut read_buf: [u8; 24] = [0; 24];
                                        let num_read = pty_process::fd_read(0, &mut read_buf, 24) as i64;
                                        let ptr = read_buf.as_ptr() as *const c_void;
                                        let len = num_read as size_t;
                                        let mut newKbd = false;
                                        if num_read==1 {
                                            // pass keystroke along
                                            libc::write(pty.fdm.as_raw_fd(), ptr, len);
                                            if read_buf[0] == 127 {
                                                // BACKSPACE was pressed - remove one character from kbdBuffer
                                                if kbdBufferPos > 0 { kbdBufferPos = kbdBufferPos - 1; }
                                            } else {
                                                utilities::insert_in_buffer( &mut kbdBuffer,
                                                                             kbdBufferLen,
                                                                             &mut kbdBufferPos,
                                                                             &read_buf,
                                                                             len );
                                            }
                                            newKbd = true;
                                            let _ = luaGlobals.set("LUA_GLOBAL_NK",kbdBufferPos as i32);
                                        } else
                                        if num_read > 1 {
                                            // this is NOT a single key... more like an escape code... so just pass it along
                                            libc::write(pty.fdm.as_raw_fd(), ptr, len);
                                            utilities::insert_in_buffer( &mut kbdBuffer,
                                                                         kbdBufferLen,
                                                                         &mut kbdBufferPos,
                                                                         &read_buf,
                                                                         len );
                                            newKbd = true;
                                            let _ = luaGlobals.set("LUA_GLOBAL_NK",kbdBufferPos as i32);
                                        } else
                                        if num_read <= 0 {
                                            normal_exit();
                                        }
                                        if newKbd {
                                            let kbdStr = String::from_utf8_lossy(&kbdBuffer[..kbdBufferPos]);
                                            for (id, entry) in configDict.iter() {
                                                if entry.when.to_uppercase()=="KEYBOARD" {
                                                    if let Some(trigger) = &entry.trigger {
                                                        let re = Regex::new(trigger).unwrap();
                                                        if re.is_match(&kbdStr) {
                                                            kbdBufferPos = 0;
                                                            let result = lua.load(&entry.code).exec();
                                                            match result {
                                                                Ok(_) => {
                                                                }
                                                                _ => {
                                                                    println!("Lua error");
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if pty.fdm.as_raw_fd()==i {
                                        // read from the pipe ... this stuff goes to the screen
                                        let mut read_buf: [u8; 256] = [0; 256];
                                        let num_read = pty_process::fd_read(pty.fdm.as_raw_fd(), &mut read_buf, 256) as i64;
                                        if num_read>0 {
                                            utilities::insert_in_buffer( &mut captureScreenBuffer,
                                                                         captureScreenBufferLen,
                                                                         &mut captureScreenBufferPos,
                                                                         &read_buf,
                                                                         num_read as size_t );

                                            let scrnStr = String::from_utf8_lossy(&captureScreenBuffer[..captureScreenBufferPos]);

                                            //  search for a matching trigger
                                            let mut foundMatch = false;
                                            for (id, entry) in configDict.iter() {
                                                if entry.when.to_uppercase()=="TERM" {
                                                    if let Some(trigger) = &entry.trigger {
                                                        let re = Regex::new(trigger).unwrap();
                                                        if let Some(mat) = re.find(&scrnStr) {
                                                            let matchStart = mat.start();
                                                            let matchEnd   = mat.end();
                                                            let matchBefor = &scrnStr[..matchStart];
                                                            let matchText  = &scrnStr[matchStart..matchEnd];
                                                            let matchAfter = &scrnStr[matchEnd..];
                                                            captureScreenBufferPos = 0;

                                                            // send initial chunk...
                                                            // TODO: what if we match several times?
                                                            let mut mB = matchBefor.to_owned();
                                                            let ptr = mB.as_mut_ptr() as *const c_void;
                                                            libc::write(0, ptr, matchBefor.len());

                                                            // execute Lua script
                                                            let _ = luaGlobals.set("LUA_GLOBAL_MATCH",matchText);
                                                            let _ = lua.load(&entry.code).exec();
                                                            foundMatch = true;

                                                            // send final chunk...
                                                            // TODO: what if we match several times?
                                                            let mut mA = matchAfter.to_owned();
                                                            let ptr = mA.as_mut_ptr() as *const c_void;
                                                            libc::write(0, ptr, matchAfter.len());
                                                        }
                                                    }
                                                }
                                            }

                                            if !foundMatch {
                                                // send stuff to screen as usual...
                                                let ptr = read_buf.as_mut_ptr() as *const c_void;
                                                let len = num_read as size_t;
                                                libc::write(0, ptr, len);
                                            }
                                        } else {
                                            normal_exit();
                                        }
                                    }
                                } else {
                                    // timeout
                                }
                            }
                        }
                        Err(_) => {
                            // select was interrupted - adjust terminal window size
                            let w = pty_process::Winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
                            ioctl(STDOUT_FILENO, TIOCGWINSZ, &w);
                            ioctl(pty.fdm.as_raw_fd(), TIOCSWINSZ, &w);
                        }
                    }
                }
            }
        }
        ForkResult::Child => {
            // This is the child process
            drop(pty.fdm);
            unsafe {
                libc::close(STDIN_FILENO);
                libc::close(STDOUT_FILENO);
                libc::close(STDERR_FILENO);
                libc::dup(pty.fds);
                libc::dup(pty.fds);
                libc::dup(pty.fds);
                libc::close(pty.fds);
                setsid();
                ioctl(0, TIOCSCTTY as u64, 1);
                ioctl(STDOUT_FILENO, TIOCSWINSZ, &w);
            }

            let mut cmd = Command::new(execProg);
            /////////////////////////////////////////////
            // TODO: make these arguments to be passed //
            //       by cmd line, on call to minmux    //
            /////////////////////////////////////////////
            //cmd.env("PS1", "[\\u@\\h \\W]$ ");
            //cmd.arg("-l");
            cmd.exec();
        }
    }
    println!("\n\nTHIS SHOULD NEVER HAPPEN!\n\n");
}
