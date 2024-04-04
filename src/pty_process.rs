// use encoding_rs::WINDOWS_1252;
use libc::c_int;
use libc::c_ushort;
use libc::c_void;
use libc::TCSANOW;
use nix::fcntl::{open, OFlag};
use nix::libc::STDIN_FILENO;
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt, PtyMaster};
use nix::sys::signal;
use nix::sys::stat::Mode;
use std::path::Path;
pub struct PTY {
    pub fdm: PtyMaster,
    pub fds: i32,
}
#[repr(C)]
pub struct Winsize {
    pub ws_row: c_ushort,    // rows, in characters
    pub ws_col: c_ushort,    // columns, in characters
    pub ws_xpixel: c_ushort, // horizontal size, pixels
    pub ws_ypixel: c_ushort, // vertical size, pixels
}

#[cfg(target_os = "freebsd")]
pub static mut SAVE_TERM: libc::termios = libc::termios {
    c_iflag: 0,
    c_oflag: 0,
    c_cflag: 0,
    c_lflag: 0,
    //c_line: 0,
    c_ispeed: 0,
    c_ospeed: 0,
    //c_cc: [0; 32],
    c_cc: [0; 20],
};

#[cfg(target_os = "linux")]
pub static mut SAVE_TERM: libc::termios = libc::termios {
    c_iflag: 0,
    c_oflag: 0,
    c_cflag: 0,
    c_lflag: 0,
    c_line: 0,
    c_ispeed: 0,
    c_ospeed: 0,
    c_cc: [0; 32],
};

pub fn fd_read(fd: c_int, buf: &mut [u8], bufSize: usize) -> usize {
    unsafe { libc::read(fd, buf.as_mut_ptr() as *mut c_void, bufSize) as usize }
}

// A function that creates a PTY pair and returns it
pub fn new() -> PTY {
    // Open the master end of the PTY
    let master =
        posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY).expect("Error[01]: Failed to opnepty");

    // Grant access to the slave end of the PTY
    grantpt(&master).unwrap_or_else(|_| {
        println!("Error[02]: Unable to grantpt");
    });
    unlockpt(&master).unwrap_or_else(|_| {
        println!("Error[03]: Unable to unlockpt");
    });
    let slave_name = unsafe { ptsname(&master) }.unwrap();
    let fds = open(
        Path::new(&slave_name),
        OFlag::O_RDWR | OFlag::O_NOCTTY,
        Mode::empty(),
    )
    .expect("Error[04]");

    let fdm = master;
    PTY { fdm, fds }
}

#[no_mangle]
pub extern "C" fn restore_term() {
    unsafe {
        libc::tcsetattr(STDIN_FILENO, TCSANOW, &SAVE_TERM);
    }
}

pub extern "C" fn handle(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    // this will cause an interruption to select()
}

pub fn install_signal_handler() -> bool {
    let result = unsafe {
        let sig_action = signal::SigAction::new(
            signal::SigHandler::SigAction(handle),
            signal::SaFlags::empty(),
            signal::SigSet::empty(),
        );
        signal::sigaction(signal::SIGWINCH, &sig_action)
    };

    if let Err(_) = result {
        print!("Fail to install signal handlers");
        return false;
    }

    return true;
}

pub fn replace_last_element_greater_than_zero(array: &mut [u8; 256]) {
    if let Some(index) = array.iter().rposition(|&x| x > 0) {
        for i in index..array.len() {
            array[i] = 0;
        }
    }
}

// pub fn read_file_string(filepath: &str) -> Result<bool, Box<dyn Error>> {
//     let data = fs::read(filepath)?;

//     let (data, _, _) = WINDOWS_1252.decode(&data);
//     Ok(data.contains("DWORD PTR") || data.contains("set disassembly-flavor intel"))
// }
