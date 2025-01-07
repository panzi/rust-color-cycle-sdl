pub mod image_to_ansi;
pub mod color;
pub mod image;
pub mod palette;
pub mod read;

use std::time::{Duration, Instant};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::mem::MaybeUninit;

use color::Rgb;
use image::{CycleImage, RgbImage};
use image_to_ansi::image_to_ansi_into;
use libc;

pub struct NBTerm;

impl NBTerm {
    pub fn new() -> std::io::Result<Self> {
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
            //ttystate.c_lflag &= !libc::ICANON;

            // minimum of number input read.
            ttystate.c_cc[libc::VMIN] = 0;
            ttystate.c_cc[libc::VTIME] = 0;

            let res = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, ttystate);
            if res == -1 {
                let err = std::io::Error::last_os_error();
                return Err(err);
            }
        }

        // CSI ?  7 l     No Auto-Wrap Mode (DECAWM), VT100.
        // CSI ? 25 l     Hide cursor (DECTCEM), VT220
        // CSI 2 J        Clear entire screen
        print!("\x1B[?25l\x1B[?7l\x1B[2J");

        Ok(Self)
    }
}

impl Drop for NBTerm {
    fn drop(&mut self) {
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

        // CSI 0 m        Reset or normal, all attributes become turned off
        // CSI ?  7 h     Auto-Wrap Mode (DECAWM), VT100
        // CSI ? 25 h     Show cursor (DECTCEM), VT220
        println!("\x1B[0m\x1B[?25h\x1B[?7h");
    }
}

fn interruptable_sleep(duration: Duration) -> bool {
    #[cfg(target_family = "unix")]
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

    #[cfg(not(target_family = "unix"))]
    {
        std::thread::sleep(duration);
        return true;
    }
}

/*
fn nb_read_avail(mut reader: impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    match reader.read(buf) {
        Err(err) => {
            match err.kind() {
                std::io::ErrorKind::WouldBlock => Ok(0),
                std::io::ErrorKind::Other if err.raw_os_error() == Some(libc::EAGAIN) => Ok(0),
                std::io::ErrorKind::Interrupted => {
                    buf[0] = b'q';
                    return Ok(1);
                },
                _ => Err(err)
            }
        }
        value => value
    }
}
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadByte {
    Quit,
    NoData,
    Byte(u8),
}

fn nb_read_byte(mut reader: impl Read) -> std::io::Result<ReadByte> {
    let mut buf = [0u8];
    match reader.read(&mut buf) {
        Err(err) => {
            match err.kind() {
                std::io::ErrorKind::WouldBlock => Ok(ReadByte::NoData),
                std::io::ErrorKind::Other if err.raw_os_error() == Some(libc::EAGAIN) => Ok(ReadByte::NoData),
                std::io::ErrorKind::Interrupted => Ok(ReadByte::Quit),
                _ => Err(err)
            }
        }
        Ok(count) => if count == 0 {
            Ok(ReadByte::NoData)
        } else {
            Ok(ReadByte::Byte(buf[0]))
        }
    }
}

fn main() -> std::io::Result<()> {
    // fcntl(0, F_SETFL, fcntl(0, GETFL) | O_NONBLOCK)
    // see also: https://web.archive.org/web/20170407122137/http://cc.byexamples.com/2007/04/08/non-blocking-user-input-in-loop-without-ncurses/
    // see also: https://stackoverflow.com/questions/717572/how-do-you-do-non-blocking-console-i-o-on-linux-in-c

    let mut args = std::env::args_os();
    let _ = args.next();
    let Some(filename) = args.next() else {
        return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
    };

    let file = File::open(Path::new(&filename))?;
    let reader = BufReader::new(file);

    let cycle_image: CycleImage = serde_json::from_reader(reader)?;

    let nbterm = NBTerm::new()?;
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    let frame_duration = Duration::from_secs_f64(1.0 / 12.0);
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

    let mut viewport = cycle_image.get_rect(
        0, 0,
        img_width.min(term_width),
        img_height.min(term_height));

    // TODO: resize prev_frame on window size or x/y pos change
    let mut prev_frame = RgbImage::new(term_width, term_height);

    // initial blank screen
    // simple_image_to_ansi_into(&prev_frame, &mut linebuf);
    let _ = write!(stdout, "\x1B[1;1H\x1B[38;2;0;0;0m\x1B[48;2;0;0;0m\x1B[2J");
    let _ = stdout.flush();

    let mut x = 0;
    let mut y = 0;
    let program_start_ts = Instant::now();
    let mut old_term_width = term_width;
    let mut old_term_height = term_height;

    loop {
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

        if img_width <= term_width {
            x = 0;
        }

        if img_height <= term_height {
            y = 0;
        }

        loop {
            match nb_read_byte(&mut stdin)? {
                ReadByte::Quit => return Ok(()),
                ReadByte::NoData => break,
                ReadByte::Byte(b'q') => return Ok(()),
                ReadByte::Byte(0x1b) => {
                    match nb_read_byte(&mut stdin)? {
                        ReadByte::Quit => return Ok(()),
                        ReadByte::NoData => break,
                        ReadByte::Byte(b'[') => {
                            match nb_read_byte(&mut stdin)? {
                                ReadByte::Quit => return Ok(()),
                                ReadByte::NoData => break,
                                ReadByte::Byte(b'A') => {
                                    // Up
                                    if img_height > term_height && y > 0 {
                                        y -= 1;
                                    }
                                }
                                ReadByte::Byte(b'B') => {
                                    // Down
                                    if img_height > term_height && y < (img_height - term_height) {
                                        y += 1;
                                    }
                                }
                                ReadByte::Byte(b'C') => {
                                    // Right
                                    if img_width > term_width && x < (img_width - term_width) {
                                        x += 1;
                                    }
                                }
                                ReadByte::Byte(b'D') => {
                                    // Left
                                    if img_width > term_width && x > 0 {
                                        x -= 1;
                                    }
                                }
                                ReadByte::Byte(b'1') => {
                                    match nb_read_byte(&mut stdin)? {
                                        ReadByte::Quit => return Ok(()),
                                        ReadByte::NoData => break,
                                        ReadByte::Byte(b'~') => {
                                            // Home
                                            if img_width > term_width {
                                                x = 0;
                                            }
                                            if img_height > term_height {
                                                y = 0;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                ReadByte::Byte(b'4') => {
                                    match nb_read_byte(&mut stdin)? {
                                        ReadByte::Quit => return Ok(()),
                                        ReadByte::NoData => break,
                                        ReadByte::Byte(b'~') => {
                                            // End
                                            if img_width > term_width {
                                                x = img_width - term_width;
                                            }
                                            if img_height > term_height {
                                                y = img_height - term_height;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                ReadByte::Byte(b'5') => {
                                    match nb_read_byte(&mut stdin)? {
                                        ReadByte::Quit => return Ok(()),
                                        ReadByte::NoData => break,
                                        ReadByte::Byte(b'A') => {
                                            // Ctrl+Up
                                            // XXX
                                            if img_height > term_height {
                                                y = 0;
                                            }
                                        }
                                        ReadByte::Byte(b'B') => {
                                            // Ctrl+Down
                                            // XXX
                                            if img_height > term_height {
                                                y = img_height - term_height;
                                            }
                                        }
                                        ReadByte::Byte(b'C') => {
                                            // Ctrl+Right
                                            // XXX
                                            if img_width > term_width {
                                                x = img_width - term_width;
                                            }
                                        }
                                        ReadByte::Byte(b'D') => {
                                            // Ctrl+Left
                                            // XXX
                                            if img_width > term_width {
                                                x = 0;
                                            }
                                        }
                                        ReadByte::Byte(b'~') => {
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
                                        _ => {}
                                    }
                                }
                                ReadByte::Byte(b'6') => {
                                    match nb_read_byte(&mut stdin)? {
                                        ReadByte::Quit => return Ok(()),
                                        ReadByte::NoData => break,
                                        ReadByte::Byte(b'~') => {
                                            // Page Down
                                            if img_height > term_height {
                                                let half = term_height / 2;
                                                let max = img_height - term_height;
                                                if y < max - half {
                                                    y += half;
                                                } else {
                                                    y = max;
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

        // render frame
        if old_x != x || old_y != y || old_term_width != term_width || old_term_height != term_height {
            viewport.get_rect_from(x, y, term_width, term_height, &cycle_image);
            prev_frame.resize(viewport.width(), viewport.height(), Rgb([0, 0, 0]));
        }

        // print!(".");
        // let _ = stdout.flush();
        image_to_ansi_into(&prev_frame, viewport.rgb_image(), true, &mut linebuf);
        //simple_image_to_ansi_into(viewport.rgb_image(), &mut linebuf);

        viewport.swap_image_buffer(&mut prev_frame);
        //eprintln!("{}", linebuf);

        let _ = write!(stdout, "\x1B[1;1H{linebuf}");
        let _ = stdout.flush();

        viewport.next_frame((frame_start_ts - program_start_ts).as_secs_f64());

        // sleep for rest of frame
        let elapsed = frame_start_ts.elapsed();
        if frame_duration > elapsed && !interruptable_sleep(frame_duration - elapsed) {
            break;
        }

        old_term_width  = term_width;
        old_term_height = term_height;

        //break;
    }

    drop(nbterm);

    Ok(())
}
