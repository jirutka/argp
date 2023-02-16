// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>

// It's based on https://github.com/clap-rs/term_size-rs/blob/master/src/platform/unix.rs.

use std::mem::zeroed;
use std::os::raw::{c_int, c_ulong, c_ushort};

static STDOUT_FILENO: c_int = 1;

// Unfortunately the actual command is not standardised...
#[cfg(any(target_os = "linux", target_os = "android"))]
static TIOCGWINSZ: c_ulong = 0x5413;

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "bitrig",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
static TIOCGWINSZ: c_ulong = 0x40087468;

#[cfg(target_os = "solaris")]
static TIOCGWINSZ: c_ulong = 0x5468;

// This has been copied from the libc crate.
#[repr(C)]
struct winsize {
    ws_row: c_ushort,
    ws_col: c_ushort,
    ws_xpixel: c_ushort,
    ws_ypixel: c_ushort,
}

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

/// Runs the ioctl command. Returns (0, 0) if the output is not to a terminal, or
/// there is an error. (0, 0) is an invalid size to have anyway, which is why
/// it can be used as a nil value.
unsafe fn get_dimensions() -> winsize {
    let mut window: winsize = zeroed();
    let result = ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut window);

    if result != -1 {
        return window;
    }
    zeroed()
}

/// Returns width of the terminal based on the current processes' stdout, if
/// the stdout stream is actually a tty. If it is not a tty, returns `None`.
pub fn term_cols() -> Option<usize> {
    let winsize { ws_col, .. } = unsafe { get_dimensions() };

    if ws_col == 0 {
        None
    } else {
        Some(ws_col as usize)
    }
}

#[cfg(test)]
mod test {
    // Compare with the output of `stty size`
    // This has been copied from https://github.com/eminence/terminal-size/blob/master/src/unix.rs.
    #[test]
    fn compare_with_stty() {
        use std::process::{Command, Stdio};

        let output = if cfg!(target_os = "linux") {
            Command::new("stty")
                .arg("size")
                .arg("-F")
                .arg("/dev/stderr")
                .stderr(Stdio::inherit())
                .output()
                .unwrap()
        } else {
            Command::new("stty")
                .arg("-f")
                .arg("/dev/stderr")
                .arg("size")
                .stderr(Stdio::inherit())
                .output()
                .unwrap()
        };
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout).unwrap();
        println!("stty: {}", stdout);

        // stdout is "rows cols"
        let mut data = stdout.split_whitespace();
        let expected: usize = str::parse(data.nth(1).unwrap()).unwrap();
        println!("cols: {}", expected);

        if let Some(actual) = super::term_cols() {
            assert_eq!(actual, expected);
        // This may happen e.g. on CI.
        } else if expected == 0 {
            eprintln!("WARN: stty reports cols 0, skipping test");
        } else {
            panic!("term_cols() return None");
        }
    }
}
