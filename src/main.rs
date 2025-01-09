// color-cycle - render color cycle images on the terminal
// Copyright (C) 2025  Mathias Panzenböck
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
// 
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

pub mod image_to_ansi;
pub mod color;
pub mod image;
pub mod palette;
pub mod read;

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::{BufReader, Read, Write};

#[cfg(not(windows))]
use std::mem::MaybeUninit;

use clap::Parser;
use image::{CycleImage, RgbImage};
use image_to_ansi::{image_to_ansi_into, simple_image_to_ansi_into};

#[cfg(not(windows))]
use libc;

const MAX_FPS: u32 = 10_000;

pub struct NBTerm;

impl NBTerm {
    pub fn new() -> std::io::Result<Self> {
        #[cfg(not(windows))]
        unsafe {
            let mut ttystate = MaybeUninit::<libc::termios>::zeroed();
            let res = libc::tcgetattr(libc::STDIN_FILENO, ttystate.as_mut_ptr());
            if res == -1 {
                let err = std::io::Error::last_os_error();
                return Err(err);
            }

            let ttystate = ttystate.assume_init_mut();

            // turn off canonical mode
            ttystate.c_lflag &= !(libc::ICANON | libc::ECHO);

            // minimum of number input read.
            ttystate.c_cc[libc::VMIN] = 0;
            ttystate.c_cc[libc::VTIME] = 0;

            let res = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, ttystate);
            if res == -1 {
                let err = std::io::Error::last_os_error();
                return Err(err);
            }
        }

//        #[cfg(windows)]
//        unsafe {
//            use winapi::shared::minwindef::{DWORD, FALSE};
//
//            let handle = winapi::um::processenv::GetStdHandle(winapi::um::winbase::STD_INPUT_HANDLE);
//            if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
//                let err = std::io::Error::last_os_error();
//                return Err(err);
//            }
//
//            let mut mode: DWORD = 0;
//
//            if winapi::um::consoleapi::GetConsoleMode(handle, &mut mode as *mut DWORD) == FALSE {
//                let err = std::io::Error::last_os_error();
//                return Err(err);
//            }
//
//            if winapi::um::consoleapi::SetConsoleMode(handle, mode & !(winapi::um::wincon::ENABLE_ECHO_INPUT | winapi::um::wincon::ENABLE_LINE_INPUT)) == FALSE {
//                let err = std::io::Error::last_os_error();
//                return Err(err);
//            }
//        }

        // CSI ?  7 l     No Auto-Wrap Mode (DECAWM), VT100.
        // CSI ? 25 l     Hide cursor (DECTCEM), VT220
        // CSI 2 J        Clear entire screen
        print!("\x1B[?25l\x1B[?7l\x1B[2J");

        Ok(Self)
    }
}

impl Drop for NBTerm {
    fn drop(&mut self) {
        #[cfg(not(windows))]
        unsafe {
            let mut ttystate = MaybeUninit::<libc::termios>::zeroed();
            let res = libc::tcgetattr(libc::STDIN_FILENO, ttystate.as_mut_ptr());
            if res == 0 {
                let ttystate = ttystate.assume_init_mut();

                // turn on canonical mode
                ttystate.c_lflag |= libc::ICANON | libc::ECHO;

                let _ = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, ttystate);
            }
        }

//        #[cfg(windows)]
//        unsafe {
//            use winapi::shared::minwindef::{DWORD, FALSE};
//            let handle = winapi::um::processenv::GetStdHandle(winapi::um::winbase::STD_INPUT_HANDLE);
//            if handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {
//                let mut mode: DWORD = 0;
//
//                if winapi::um::consoleapi::GetConsoleMode(handle, &mut mode as *mut DWORD) != FALSE {
//                    winapi::um::consoleapi::SetConsoleMode(handle, mode | winapi::um::wincon::ENABLE_ECHO_INPUT | winapi::um::wincon::ENABLE_LINE_INPUT);
//                }
//            }
//        }

        // CSI 0 m        Reset or normal, all attributes become turned off
        // CSI ?  7 h     Auto-Wrap Mode (DECAWM), VT100
        // CSI ? 25 h     Show cursor (DECTCEM), VT220
        println!("\x1B[0m\x1B[?25h\x1B[?7h");
    }
}

fn interruptable_sleep(duration: Duration) -> bool {
    #[cfg(unix)]
    {
        let nanos = duration.as_nanos();
        let sec = nanos / 1_000_000_000u128;
        let req = libc::timespec {
            tv_sec:  sec as i64,
            tv_nsec: (nanos - (sec * 1_000_000_000u128)) as i64,
        };
        let ret = unsafe { libc::nanosleep(&req, std::ptr::null_mut()) };
        return ret == 0;
    }

    #[cfg(not(unix))]
    {
        std::thread::sleep(duration);
        return true;
    }
}

#[cfg(windows)]
extern {
    fn _getch() -> core::ffi::c_char;
    fn _kbhit() -> core::ffi::c_int;
}

#[cfg(windows)]
fn nb_read_byte(mut _reader: impl Read) -> std::io::Result<Option<u8>> {
    unsafe {
        if _kbhit() == 0 {
            return Ok(None);
        }

        let ch = _getch();
        Ok(Some(ch as u8))
    }
}

#[cfg(not(windows))]
fn nb_read_byte(mut reader: impl Read) -> std::io::Result<Option<u8>> {
    let mut buf = [0u8];
    loop {
        return match reader.read(&mut buf) {
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::WouldBlock => Ok(None),

                    #[cfg(not(windows))]
                    std::io::ErrorKind::Other if err.raw_os_error() == Some(libc::EAGAIN) => Ok(None),

                    std::io::ErrorKind::Interrupted => continue,
                    _ => Err(err)
                }
            }
            Ok(count) => if count == 0 {
                Ok(None)
            } else {
                Ok(Some(buf[0]))
            }
        };
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, after_help = "\
color-cycle  Copyright (C) 2025  Mathias Panzenböck
License: GPL-3.0
Bugs: https://github.com/panzi/rust-color-cycle/issues"
)]
pub struct Args {
    /// Frames per second.
    /// 
    /// Attempt to render in this number of frames per second.
    /// Actual FPS might be lower.
    #[arg(short, long, default_value_t = 25, value_parser = clap::value_parser!(u32).range(1..MAX_FPS as i64))]
    pub fps: u32,

    /// Enable blend mode.
    /// 
    /// This blends the animated color palette for smoother display.
    #[arg(short, long, default_value_t = false)]
    pub blend: bool,

    /// Enable On Screen Display.
    /// 
    /// Displas messages when changing things like blend mode or FPS.{n}
    #[arg(short, long, default_value_t = false)]
    pub osd: bool,

    /// Path to a Canvas Cycle JSON file.
    #[arg()]
    pub path: OsString,
}

fn main() -> std::io::Result<()> {
    let mut args = Args::parse();

    let file = File::open(args.path)?;
    let reader = BufReader::new(file);

    let cycle_image: CycleImage = serde_json::from_reader(reader)?;

    let _nbterm = NBTerm::new()?;
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    let mut frame_duration = Duration::from_secs_f64(1.0 / (args.fps as f64));
    let mut linebuf = String::new();

    let img_width = cycle_image.width();
    let img_height = cycle_image.height();
    let (term_width, term_height) = {
        let term_size = term_size::dimensions();
        if let Some((columns, rows)) = term_size {
            (columns as u32, rows as u32 * 2)
        } else {
            (img_width, img_height)
        }
    };

    // initial blank screen
    let _ = write!(stdout, "\x1B[1;1H\x1B[38;2;0;0;0m\x1B[48;2;0;0;0m\x1B[2J");
    let _ = stdout.flush();

    let mut x = 0;
    let mut y = 0;

    if img_width > term_width {
        x = (img_width - term_width) / 2;
    }

    if img_height > term_height {
        y = (img_height - term_height) / 2;
    }

    let mut viewport = cycle_image.get_rect(
        x, y,
        img_width.min(term_width),
        img_height.min(term_height));

    let mut prev_frame = RgbImage::new(viewport.width(), viewport.height());

    let mut old_term_width = term_width;
    let mut old_term_height = term_height;

    let running = Arc::new(AtomicBool::new(true));

    {
        let running = running.clone();
        let _ = ctrlc::set_handler(move || {
            running.store(false, Ordering::Relaxed);
        });
    }

    let mut message = String::new();
    let mut message_shown = false;
    let message_display_duration = Duration::from_secs(3);

    let loop_start_ts = Instant::now();
    let mut message_end_ts = loop_start_ts;

    while running.load(Ordering::Relaxed) {
        let frame_start_ts = Instant::now();

        // process input
        let term_size = term_size::dimensions();
        let (term_width, term_height) = if let Some((columns, rows)) = term_size {
            (columns as u32, rows as u32 * 2)
        } else {
            (img_width, img_height)
        };

        let old_x = x;
        let old_y = y;

        let mut viewport_x = 0;
        let mut viewport_y = 0;

        if img_width <= term_width {
            x = 0;
            viewport_x = (term_width - img_width) / 2;
        } else if x > img_width - term_width {
            x = img_width - term_width;
        }

        if img_height <= term_height {
            y = 0;
            viewport_y = (term_height - img_height) / 2;
        } else if y > img_height - term_height {
            y = img_height - term_height;
        }

        let mut updated_message = false;
        macro_rules! show_message {
            ($($args:expr),+) => {
                if args.osd {
                    message_end_ts = frame_start_ts + message_display_duration;
                    message.clear();
                    use std::fmt::Write;
                    message.push_str(" ");
                    let _ = write!(&mut message, $($args),+);
                    message.push_str(" ");
                    updated_message = true;
                }
            };
        }

        loop {
            // TODO: Windows support, maybe with ReadConsoleInput()?
            match nb_read_byte(&mut stdin)? {
                None => break,
                Some(b'q') => return Ok(()),
                Some(b'b') => {
                    args.blend = !args.blend;

                    show_message!("Blend Mode: {}", if args.blend { "Enabled" } else { "Disabled" });
                }
                Some(b'o') => {
                    if args.osd {
                        show_message!("OSD: Disabled");
                        args.osd = false;
                    } else {
                        args.osd = true;
                        show_message!("OSD: Enabled");
                    }
                }
                Some(b'+') => {
                    if args.fps < MAX_FPS {
                        args.fps += 1;
                        frame_duration = Duration::from_secs_f64(1.0 / args.fps as f64);

                        show_message!("FPS: {}", args.fps);
                    }
                }
                Some(b'-') => {
                    if args.fps > 1 {
                        args.fps -= 1;
                        frame_duration = Duration::from_secs_f64(1.0 / args.fps as f64);

                        show_message!("FPS: {}", args.fps);
                    }
                }
                Some(0x1b) => {
                    match nb_read_byte(&mut stdin)? {
                        None => return Ok(()),
                        Some(0x1b) => return Ok(()),
                        Some(b'[') => {
                            match nb_read_byte(&mut stdin)? {
                                None => break,
                                Some(b'A') => {
                                    // Up
                                    if img_height > term_height && y > 0 {
                                        y -= 1;
                                    }
                                }
                                Some(b'B') => {
                                    // Down
                                    if img_height > term_height && y < (img_height - term_height) {
                                        y += 1;
                                    }
                                }
                                Some(b'C') => {
                                    // Right
                                    if img_width > term_width && x < (img_width - term_width) {
                                        x += 1;
                                    }
                                }
                                Some(b'D') => {
                                    // Left
                                    if img_width > term_width && x > 0 {
                                        x -= 1;
                                    }
                                }
                                Some(b'H') => {
                                    // Home
                                    if img_width > term_width {
                                        x = 0;
                                    }
                                }
                                Some(b'F') => {
                                    // End
                                    if img_width > term_width {
                                        x = img_width - term_width;
                                    }
                                }
                                Some(b'1') => {
                                    match nb_read_byte(&mut stdin)? {
                                        None => break,
                                        Some(b';') => {
                                            match nb_read_byte(&mut stdin)? {
                                                None => break,
                                                Some(b'5') => {
                                                    match nb_read_byte(&mut stdin)? {
                                                        None => break,
                                                        Some(b'H') => {
                                                            // Ctrl+Home
                                                            if img_height > term_height {
                                                                y = 0;
                                                            }
                                                        }
                                                        Some(b'F') => {
                                                            // Ctrl+End
                                                            if img_height > term_height {
                                                                y = img_height - term_height;
                                                            }
                                                        }
                                                        _ => break,
                                                    }
                                                }
                                                _ => break,
                                            }
                                        }
                                        _ => break,
                                    }
                                }
                                Some(b'5') => {
                                    match nb_read_byte(&mut stdin)? {
                                        None => break,
                                        Some(b'~') => {
                                            // Page Up
                                            if img_height > term_height {
                                                let half = term_height / 2;
                                                if y > half {
                                                    y -= half;
                                                } else {
                                                    y = 0;
                                                }
                                            }
                                        }
                                        Some(b';') => {
                                            match nb_read_byte(&mut stdin)? {
                                                None => break,
                                                Some(b'3') => {
                                                    match nb_read_byte(&mut stdin)? {
                                                        None => break,
                                                        Some(b'~') => {
                                                            // Alt+Page Up
                                                            if img_width > term_width {
                                                                let half = term_width / 2;
                                                                if x > half {
                                                                    x -= half;
                                                                } else {
                                                                    x = 0;
                                                                }
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Some(b'6') => {
                                    match nb_read_byte(&mut stdin)? {
                                        None => break,
                                        Some(b'~') => {
                                            // Page Down
                                            if img_height > term_height {
                                                let half = term_height / 2;
                                                let max_y = img_height - term_height;
                                                y += half;
                                                if y > max_y {
                                                    y = max_y;
                                                }
                                            }
                                        }
                                        Some(b';') => {
                                            match nb_read_byte(&mut stdin)? {
                                                None => break,
                                                Some(b'3') => {
                                                    match nb_read_byte(&mut stdin)? {
                                                        None => break,
                                                        Some(b'~') => {
                                                            // Alt+Page Down
                                                            if img_width > term_width {
                                                                let half = term_width / 2;
                                                                let max_x = img_width - term_width;
                                                                x += half;
                                                                if x > max_x {
                                                                    x = max_x;
                                                                }
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // render frame
        let mut full_redraw = false;
        let viewport_row = viewport_y / 2 + 1;
        let viewport_column = viewport_x + 1;
        if old_x != x || old_y != y || old_term_width != term_width || old_term_height != term_height {
            viewport.get_rect_from(x, y, term_width, term_height, &cycle_image);

            if old_term_width != term_width || old_term_height != term_height {
                prev_frame = RgbImage::new(viewport.width(), viewport.height());
                full_redraw = true;

                //let _ = write!(stdout, "\x1B[38;2;0;0;0m\x1B[48;2;0;0;0m\x1B[2J");
                if viewport.width() < term_width || viewport.height() < term_height {
                    let _ = write!(stdout, "\x1B[38;2;0;0;0m\x1B[48;2;0;0;0m");

                    if viewport_y > 0 {
                        let _ = write!(stdout, "\x1B[{};1H\x1B[1J", viewport_row);
                    }

                    let viewport_rows = (viewport.height() + 1) / 2;
                    let viewport_end_row = viewport_row + viewport_rows;
                    if viewport_x > 0 {
                        let column = viewport_column - 1;
                        for row in viewport_row..viewport_end_row {
                            let _ = write!(stdout, "\x1B[{};{}H\x1B[1K", row, column);
                        }
                    }

                    if viewport_x + viewport.width() < term_width {
                        let viewport_end_column = viewport_column + viewport.width();
                        for row in viewport_row..viewport_end_row {
                            let _ = write!(stdout, "\x1B[{};{}H\x1B[0K", row, viewport_end_column);
                        }
                    }

                    if (viewport_y + viewport.height() + 1) / 2 < term_height / 2 {
                        let _ = write!(stdout, "\x1B[{};1H\x1B[0J", viewport_end_row);
                    }
                }
            }
        }

        viewport.render_frame((frame_start_ts - loop_start_ts).as_secs_f64(), args.blend);

        let full_width = viewport.width() >= term_width;
        if full_redraw {
            simple_image_to_ansi_into(viewport.rgb_image(), &mut linebuf);
        } else {
            image_to_ansi_into(&prev_frame, viewport.rgb_image(), full_width, &mut linebuf);
        }

        viewport.swap_image_buffer(&mut prev_frame);

        let _ = write!(stdout, "\x1B[{};{}H{linebuf}", viewport_row, viewport_column);

        old_term_width  = term_width;
        old_term_height = term_height;

        if message_end_ts >= frame_start_ts {
            if updated_message {
                // full redraw next frame by faking old term size of 0x0
                old_term_width  = 0;
                old_term_height = 0;
            } else {
                let msg_len = message.len();

                let column = if msg_len < term_width as usize {
                    (term_width as usize - msg_len) / 2 + 1
                } else { 1 };

                let message = if msg_len > term_width as usize {
                    &message[..term_width as usize]
                } else {
                    &message
                };

                let _ = write!(stdout,
                    "\x1B[{};{}H\x1B[38;2;255;255;255m\x1B[48;2;0;0;0m{}",
                    term_height, column, message);
                message_shown = true;
            }
        } else if message_shown {
            // full redraw next frame by faking old term size of 0x0
            old_term_width  = 0;
            old_term_height = 0;
            message_shown = false;
        }

        let _ = stdout.flush();

        // sleep for rest of frame
        let elapsed = frame_start_ts.elapsed();
        if frame_duration > elapsed && !interruptable_sleep(frame_duration - elapsed) {
            break;
        }
    }

    Ok(())
}
