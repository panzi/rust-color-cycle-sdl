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

pub mod color;
pub mod image;
pub mod palette;
pub mod read;

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::BufReader;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use sdl2::rwops::RWops;
use sdl2::ttf::Font;
use sdl2::video::{FullscreenType, WindowPos};

#[cfg(not(windows))]
use std::mem::MaybeUninit;

use clap::Parser;
use image::{LivingWorld, RgbImage};

#[cfg(not(windows))]
use libc;

const MAX_FPS: u32 = 10_000;
const TIME_STEP: u64 = 60 * 5 * 1000;
const DAY_DURATION: u64 = 24 * 60 * 60 * 1000;
const FAST_FORWARD_SPEED: u64 = 10_000;

const HACK_FONT: &[u8] = include_bytes!("../assets/Hack-Regular.ttf");

fn interruptable_sleep(duration: Duration) -> bool {
    #[cfg(unix)]
    {
        let req = libc::timespec {
            tv_sec:  duration.as_secs() as libc::time_t,
            tv_nsec: duration.subsec_nanos() as i64,
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

    /// Show list of hotkeys.
    #[arg(long, default_value_t = false)]
    pub help_hotkeys: bool,

    /// Path to a Canvas Cycle JSON file.
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();

    if args.help_hotkeys {
        println!("\
Hotkeys
=======
B              Toggle blend mode
Q or Esccape   Quit program
O              Toggle On Screen Display of message
N              Open next file
P              Open previous file
1 to 9         Open file by index
0              Open last file
+              Increase frames per second by 1
-              Decrease frames per second by 1
F              Toggle full-screen
W              Toogle fast forward ({FAST_FORWARD_SPEED}x speed).
A              Go back in time by 5 minutes.
D              Go forward in time by 5 minutes.
S              Go to current time and continue normal progression.");
        return;
    }

    match ColorCycleViewer::new(ColorCycleViewerOptions {
        fps: args.fps,
        blend: args.blend,
        osd: args.osd,
        paths: args.paths,
    }) {
        Ok(mut viewer) => {
            if let Err(err) = viewer.run() {
                eprintln!("{}: {}", viewer.paths[viewer.file_index].to_string_lossy(), err);
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);

        }
    }
}

#[derive(Debug, Clone, Default)]
struct ColorCycleViewerOptions {
    fps: u32,
    blend: bool,
    osd: bool,
    paths: Vec<PathBuf>,
}

struct ColorCycleViewer {
    fps: u32,
    blend: bool,
    osd: bool,
    paths: Vec<PathBuf>,
    file_index: usize,
    running: Arc<AtomicBool>,
    current_time: Option<u64>,
    time_speed: u64,

    #[allow(unused)]
    sdl: sdl2::Sdl,
    #[allow(unused)]
    video: sdl2::VideoSubsystem,
    canvas: sdl2::render::WindowCanvas,
    event_pump: sdl2::EventPump,
}

impl ColorCycleViewer {
    pub fn new(options: ColorCycleViewerOptions) -> Result<ColorCycleViewer, String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let window = video
            .window("Color Cycle Viewer", 640, 480)
            .position_centered()
            .resizable()
            .build()
            .map_err(|err| err.to_string())?;
        let event_pump = sdl.event_pump()?;

        sdl.mouse().show_cursor(false);

        let canvas = window.into_canvas()
            .accelerated()
            .build()
            .map_err(|err| err.to_string())?;

        Ok(ColorCycleViewer {
            fps: options.fps,
            blend: options.blend,
            osd: options.osd,
            paths: options.paths,
            running: Arc::new(AtomicBool::new(true)),
            current_time: None,
            time_speed: 1,
            file_index: 0,

            sdl,
            video,
            canvas,
            event_pump,
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        {
            let running = self.running.clone();
            let _ = ctrlc::set_handler(move || {
                running.store(false, Ordering::Relaxed);
            });
        }

        let ttf = sdl2::ttf::init().map_err(|err| err.to_string())?;
        let font = ttf.load_font_from_rwops(RWops::from_bytes(HACK_FONT)?, 16)?;

        let mut init = true;
        loop {
            match self.show_image(init, &font) {
                Ok(Action::Goto(index)) => {
                    self.file_index = index;
                }
                Ok(Action::Quit) => {
                    return Ok(());
                }
                Err(err) => {
                    return Err(err);
                }
            }
            init = false;
        }
    }

    fn show_image(&mut self, init: bool, font: &Font) -> Result<Action, String> {
        let path = &self.paths[self.file_index];
        let file = File::open(path).map_err(|err| err.to_string())?;
        let reader = BufReader::new(file);

        let living_world: LivingWorld = serde_json::from_reader(reader).map_err(|err| err.to_string())?;
        // TODO: implement full worlds demo support
        let cycle_image = living_world.base();
        let mut blended_palette = cycle_image.palette().clone();
        let mut cycled_palette1 = blended_palette.clone();
        let mut cycled_palette2 = blended_palette.clone();

        let mut frame_duration = Duration::from_secs_f64(1.0 / (self.fps as f64));

        let img_width = cycle_image.width();
        let img_height = cycle_image.height();
        let pitch = img_width as usize * 3;

        let mut frame = RgbImage::new(img_width, img_height);

        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator.create_texture(
            PixelFormatEnum::RGB24,
            sdl2::render::TextureAccess::Streaming,
            img_width, img_height
        ).map_err(|err| err.to_string())?;

        let filename = path.file_name().map(|f| f.to_string_lossy()).unwrap_or_else(|| path.to_string_lossy());
        self.canvas.window_mut().set_title(&format!("{filename} - Canvas Cycle Viewer")).log_error("window.set_title()");

        if init && self.canvas.window().fullscreen_state() == FullscreenType::Off {
            self.canvas.window_mut().set_size(img_width, img_height).log_error("window.set_size()");
            self.canvas.window_mut().set_position(WindowPos::Centered, WindowPos::Centered);
        }

        let mut message = String::new();
        let mut message_texture = None;
        let message_display_duration = Duration::from_secs(3);

        self.canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        self.canvas.set_integer_scale(true).log_error("canvas.set_integer_scale(true)");

        let loop_start_ts = Instant::now();
        let mut message_end_ts = if self.osd {
            message.push_str(" ");
            message.push_str(&filename);
            message.push_str(" ");
            println!("{message}");
            loop_start_ts + message_display_duration
        } else {
            loop_start_ts
        };

        while self.running.load(Ordering::Relaxed) {
            let frame_start_ts = Instant::now();
            let mut time_of_day = if let Some(current_time) = self.current_time {
                current_time
            } else {
                get_time_of_day_msec(self.time_speed)
            };

            macro_rules! show_message {
                ($($args:expr),+) => {
                    if self.osd {
                        message_end_ts = frame_start_ts + message_display_duration;
                        message.clear();
                        use std::fmt::Write;
                        message.push_str(" ");
                        let _ = write!(&mut message, $($args),+);
                        message.push_str(" ");
                        // println!("{message}");
                        message_texture = None;
                    }
                };
            }

            // process input
            for event in self.event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => return Ok(Action::Quit),
                    Event::KeyDown { keycode, .. } => {
                        if let Some(keycode) = keycode {
                            match keycode {
                                Keycode::Q | Keycode::ESCAPE => {
                                    // quit
                                    return Ok(Action::Quit);
                                }
                                Keycode::B => {
                                    // toggle blend mode
                                    self.blend = !self.blend;

                                    show_message!("Blend Mode: {}", if self.blend { "Enabled" } else { "Disabled" });
                                }
                                Keycode::O => {
                                    // toggle OSD
                                    if self.osd {
                                        show_message!("OSD: Disabled");
                                        self.osd = false;
                                    } else {
                                        self.osd = true;
                                        show_message!("OSD: Enabled");
                                    }
                                }
                                Keycode::PLUS => {
                                    // increase FPS
                                    if self.fps < MAX_FPS {
                                        self.fps += 1;
                                        frame_duration = Duration::from_secs_f64(1.0 / self.fps as f64);

                                        show_message!("FPS: {}", self.fps);
                                    }
                                }
                                Keycode::MINUS => {
                                    // decrease FPS
                                    if self.fps > 1 {
                                        self.fps -= 1;
                                        frame_duration = Duration::from_secs_f64(1.0 / self.fps as f64);

                                        show_message!("FPS: {}", self.fps);
                                    }
                                }
                                Keycode::N => {
                                    // next file
                                    let new_index = self.file_index + 1;
                                    if new_index >= self.paths.len() {
                                        show_message!("Already at last file.");
                                    } else {
                                        return Ok(Action::Goto(new_index));
                                    }
                                }
                                Keycode::P => {
                                    // previous file
                                    if self.file_index == 0 {
                                        show_message!("Already at first file.");
                                    } else {
                                        return Ok(Action::Goto(self.file_index - 1));
                                    }
                                }
                                Keycode::A => {
                                    // back in time
                                    let rem = time_of_day % TIME_STEP;
                                    let new_time = time_of_day - rem;
                                    if new_time == time_of_day {
                                        if new_time < TIME_STEP {
                                            time_of_day = DAY_DURATION - TIME_STEP;
                                        } else {
                                            time_of_day = new_time - TIME_STEP;
                                        }
                                    } else {
                                        time_of_day = new_time;
                                    }
                                    self.time_speed = 1;
                                    self.current_time = Some(time_of_day);
                                    let (hours, mins) = get_hours_mins(time_of_day);
                                    show_message!("{hours}:{mins:02}");
                                }
                                Keycode::D => {
                                    // forward in time
                                    let rem = time_of_day % TIME_STEP;
                                    let new_time = time_of_day - rem + TIME_STEP;
                                    if new_time >= DAY_DURATION {
                                        time_of_day = 0;
                                    } else {
                                        time_of_day = new_time;
                                    }
                                    self.time_speed = 1;
                                    self.current_time = Some(time_of_day);
                                    let (hours, mins) = get_hours_mins(time_of_day);
                                    show_message!("{hours}:{mins:02}");
                                }
                                Keycode::S => {
                                    // to current time
                                    if self.current_time.is_some() {
                                        self.time_speed = 1;
                                        self.current_time = None;
                                        time_of_day = get_time_of_day_msec(self.time_speed);
                                    }
                                    let (hours, mins) = get_hours_mins(time_of_day);
                                    show_message!("{hours}:{mins:02}");
                                }
                                Keycode::F => {
                                    // toggle fullscreen
                                    let window = self.canvas.window_mut();
                                    let value = match window.fullscreen_state() {
                                        FullscreenType::Desktop | FullscreenType::True => FullscreenType::Off,
                                        FullscreenType::Off => FullscreenType::Desktop,
                                    };
                                    window.set_fullscreen(value).log_error("window.set_fullscreen()");
                                }
                                Keycode::W => {
                                    // toggle fast forward
                                    if self.time_speed == 1 {
                                        self.time_speed = FAST_FORWARD_SPEED;
                                        self.current_time = None;
                                        time_of_day = get_time_of_day_msec(self.time_speed);
                                        show_message!("Fast Forward: ON");
                                    } else {
                                        self.time_speed = 1;
                                        show_message!("Fast Forward: OFF");
                                    }
                                }
                                Keycode::KP_0 | Keycode::NUM_0 => {
                                    return Ok(Action::Goto(self.paths.len() - 1));
                                }
                                Keycode::KP_1 | Keycode::NUM_1 => {
                                    return Ok(Action::Goto(0));
                                }
                                _ => {
                                    let index = if keycode.into_i32() >= Keycode::KP_2.into_i32() && keycode.into_i32() <= Keycode::KP_9.into_i32() {
                                        keycode.into_i32() - Keycode::KP_1.into_i32()
                                    } else if keycode.into_i32() >= Keycode::NUM_2.into_i32() && keycode.into_i32() <= Keycode::NUM_9.into_i32() {
                                        keycode.into_i32() - Keycode::NUM_1.into_i32()
                                    } else {
                                        0
                                    };

                                    if index > 0 {
                                        if index as usize >= self.paths.len() {
                                            show_message!("Only {} files opened!", self.paths.len());
                                        } else {
                                            return Ok(Action::Goto(index as usize));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // render frame
            let blend_cycle = (frame_start_ts - loop_start_ts).as_secs_f64();
            if !living_world.timeline().is_empty() {
                let mut palette1 = &living_world.palettes()[living_world.timeline().last().unwrap().palette_index()];
                let mut palette2 = palette1;
                let mut prev_time_of_day = 0;
                let mut next_time_of_day = 0;
    
                // TODO: binary search?
                let mut found = false;
                for event in living_world.timeline() {
                    prev_time_of_day = next_time_of_day;
                    next_time_of_day = event.time_of_day() as u64 * 1000;
                    palette1 = palette2;
                    palette2 = &living_world.palettes()[event.palette_index()];
                    if next_time_of_day > time_of_day {
                        found = true;
                        break;
                    }
                }

                if !found {
                    prev_time_of_day = next_time_of_day;
                    next_time_of_day = DAY_DURATION;
                    palette1 = palette2;
                    palette2 = &living_world.palettes()[living_world.timeline().first().unwrap().palette_index()];
                }

                let current_span = next_time_of_day - prev_time_of_day;
                let time_in_span = time_of_day - prev_time_of_day;
                let blend_palettes = time_in_span as f64 / current_span as f64;

                cycled_palette1.apply_cycles_from(palette1.palette(), palette1.cycles(), blend_cycle, self.blend);
                cycled_palette2.apply_cycles_from(palette2.palette(), palette2.cycles(), blend_cycle, self.blend);

                crate::palette::blend(&cycled_palette1, &cycled_palette2, blend_palettes, &mut blended_palette);

                cycle_image.indexed_image().apply_with_palette(&mut frame, &blended_palette);
            } else {
                cycled_palette1.apply_cycles_from(&blended_palette, cycle_image.cycles(), blend_cycle, self.blend);
                cycle_image.indexed_image().apply_with_palette(&mut frame, &cycled_palette1);
            }

            if let Err(err) = texture.update(Rect::new(0, 0, img_width, img_height), frame.data(), pitch) {
                return Err(err.to_string());
            }

            self.canvas.clear();
            let (canvas_width, canvas_height) = self.canvas.output_size()?;

            let mut draw_width = canvas_width;
            let mut draw_height = img_height * canvas_width / img_width;

            if draw_height > canvas_height {
                draw_width = img_width * canvas_height / img_height;
                draw_height = canvas_height;
            }

            let draw_x = if draw_width < canvas_width {
                ((canvas_width - draw_width) / 2) as i32
            } else { 0 };

            let draw_y = if draw_height < canvas_height {
                ((canvas_height - draw_height) / 2) as i32
            } else { 0 };

            self.canvas.copy(&texture, None, Rect::new(draw_x, draw_y, draw_width, draw_height))?;

            if self.time_speed != 1 && message.is_empty() {
                let (hours, mins) = get_hours_mins(time_of_day);
                show_message!("{hours}:{mins:02}");
            }

            if message_end_ts >= frame_start_ts {
                // draw OSD message
                let texture = if let Some(texture) = &message_texture {
                    texture
                } else {
                    let surface = font.render(&message)
                        .shaded(Color::RGB(255, 255, 255), Color::RGB(0, 0, 0))
                        .map_err(|err| err.to_string())?;

                    message_texture = Some(texture_creator
                        .create_texture_from_surface(surface)
                        .map_err(|err| err.to_string())?);

                    message_texture.as_ref().unwrap()
                };

                let TextureQuery { width, height, .. } = texture.query();

                self.canvas.copy(&texture, None, Rect::new(
                    ((canvas_width - width) / 2) as i32,
                    (canvas_height - 2 * height) as i32,
                    width, height))?;
            }

            self.canvas.present();

            // sleep for rest of frame
            let elapsed = frame_start_ts.elapsed();
            if frame_duration > elapsed && !interruptable_sleep(frame_duration - elapsed) {
                return Ok(Action::Quit);
            }
        }

        Ok(Action::Quit)
    }
}

enum Action {
    Goto(usize),
    Quit,
}

fn get_time_of_day_msec(time_speed: u64) -> u64 {
    #[cfg(not(windows))]
    unsafe {
        let mut tod = MaybeUninit::<libc::timeval>::zeroed();
        if libc::gettimeofday(tod.as_mut_ptr(), std::ptr::null_mut()) != 0 {
            return 0;
        }
        let tod = tod.assume_init_ref();
        (
            tod.tv_sec as u64 * 1000 * time_speed +
            tod.tv_usec as u64 * time_speed / 1000
        ) % DAY_DURATION
    }

    #[cfg(windows)]
    unsafe {
        let mut tm = MaybeUninit::<winapi::um::minwinbase::SYSTEMTIME>::zeroed();
        winapi::um::sysinfoapi::GetLocalTime(tm.as_mut_ptr());
        let tm = tm.assume_init_ref();

        (
            tm.wHour as u64 * 60 * 60 * 1000 +
            tm.wMinute as u64 * 60 * 1000 +
            tm.wSecond as u64 * 1000 +
            tm.wMilliseconds as u64
        ) * time_speed % DAY_DURATION
    }
}

fn get_hours_mins(time_of_day: u64) -> (u32, u32) {
    let mins = (time_of_day / (60 * 1000)) as u32;
    let hours = mins / 60;
    (hours, mins - hours * 60)
}

trait Loggable {
    fn log_error(&self, msg: &str);

    #[allow(unused)]
    fn log_warning(&self, msg: &str);

    #[allow(unused)]
    fn log_info(&self, msg: &str);
}

impl<T, E> Loggable for std::result::Result<T, E>
where E: std::fmt::Display {
    #[inline]
    fn log_error(&self, msg: &str) {
        if let Err(err) = self {
            eprint!("ERROR: {msg}: {}", err);
        }
    }

    #[inline]
    fn log_info(&self, msg: &str) {
        if let Err(err) = self {
            print!("INFO: {msg}: {}", err);
        }
    }

    #[inline]
    fn log_warning(&self, msg: &str) {
        if let Err(err) = self {
            print!("WARNING: {msg}: {}", err);
        }
    }
}
