// color-cycle - render color cycle images
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
pub mod ilbm;
pub mod bitvec;
pub mod error;

use std::fmt::{Debug, Display, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::{BufReader, Seek};
use std::u64;

use color::Rgb;
use palette::Palette;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::messagebox::{MessageBoxButtonFlag, MessageBoxFlag};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use sdl2::rwops::RWops;
use sdl2::sys::SDL_WindowFlags;
use sdl2::video::{FullscreenType, WindowPos};

#[cfg(not(windows))]
use std::mem::MaybeUninit;

use clap::Parser;
use image::{CycleImage, IndexedImage, LivingWorld};

#[cfg(not(windows))]
use libc;

const MAX_FPS: u32 = 10_000;
const TIME_STEP: u64 = 5 * 60 * 1000;
const SMALL_TIME_STEP: u64 = 60 * 1000;
const DAY_DURATION: u64 = 24 * 60 * 60 * 1000;
const FAST_FORWARD_SPEED: u64 = 10_000;

const HACK_FONT: &[u8] = include_bytes!("../assets/Hack-Regular.ttf");
const APP_NAME: &str = "Color Cycle Viewer";

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
    #[arg(short, long, default_value_t = 60, value_parser = clap::value_parser!(u32).range(1..MAX_FPS as i64))]
    pub fps: u32,

    /// Enable blend mode.
    /// 
    /// This blends the animated color palette for smoother display.
    #[arg(short, long, default_value_t = false)]
    pub blend: bool,

    /// Enable On Screen Display.
    /// 
    /// Displays messages when changing things like blend mode or FPS.{n}
    #[arg(short, long, default_value_t = false)]
    pub osd: bool,

    /// Start in fullscreen
    #[arg(short = 'F', long, default_value_t = false)]
    pub full_screen: bool,

    /// Cover the window with the animation.
    /// 
    /// Per default the animation will be contained, leading to black bars if
    /// the window doesn't have the same aspect ratio as the animation. With
    /// this option the animation is zoomed in so that it will cover the window
    /// and will crop out parts of the animation.
    #[arg(short, long, default_value_t = false)]
    pub cover: bool,

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
B                  Toggle blend mode
Q                  Quit program
Escape             Close full-screen or quit program
O                  Toggle On Screen Display
C                  Toggle zoom to cover/contain
N                  Open next file
P                  Open previous file
1 to 9             Open file by index
0                  Open last file
+                  Increase frames per second by 1
-                  Decrease frames per second by 1
F or F11           Toggle full-screen
W                  Toogle fast forward ({FAST_FORWARD_SPEED}x speed)
A                  Go back in time by 5 minutes
Shift+A            Go back in time by 1 minute
D                  Go forward in time by 5 minutes
Shift+D            Go forward in time by 1 minute
S                  Go to current time and continue normal progression
I                  Reverse pixels in columns of 8.
                   This is a hack fix for images that appear to be
                   broken like that.
Cursor Up          Move view-port up by 1 pixel
Cursor Down        Move view-port down by 1 pixel
Cursor Left        Move view-port left by 1 pixel
Cursor Right       Move view-port right by 1 pixel
Ctrl+Cursor Up     Move view-port up by 5 pixel
Ctrl+Cursor Down   Move view-port down by 5 pixel
Ctrl+Cursor Left   Move view-port left by 5 pixel
Ctrl+Cursor Right  Move view-port right by 5 pixel");
        return;
    }

    match ColorCycleViewer::new(ColorCycleViewerOptions {
        fps: args.fps,
        blend: args.blend,
        osd: args.osd,
        full_screen: args.full_screen,
        cover: args.cover,
        paths: args.paths,
        ttf: &match sdl2::ttf::init() {
            Ok(ttf) => ttf,
            Err(err) => {
                show_error(err);
                std::process::exit(1);
            }
        },
    }) {
        Ok(mut viewer) => {
            if let Err(err) = viewer.run() {
                show_error(format_args!("{}: {}", viewer.options.paths[viewer.file_index].to_string_lossy(), err));
                std::process::exit(1);
            }
        }
        Err(err) => {
            show_error(err);
            std::process::exit(1);
        }
    }
}

fn show_error(message: impl Display) {
    let message = message.to_string();
    eprintln!("{}", &message);
    let _ = sdl2::messagebox::show_message_box(
        MessageBoxFlag::ERROR, &[
            sdl2::messagebox::ButtonData {
                button_id: 0,
                flags: MessageBoxButtonFlag::ESCAPEKEY_DEFAULT | MessageBoxButtonFlag::RETURNKEY_DEFAULT,
                text: "Ok"
            }
        ], &format!("Error - {APP_NAME}"), &message, None, None);
}

struct ColorCycleViewerOptions<'font> {
    fps: u32,
    blend: bool,
    osd: bool,
    paths: Vec<PathBuf>,
    full_screen: bool,
    cover: bool,
    ttf: &'font sdl2::ttf::Sdl2TtfContext,
}

struct ColorCycleViewer<'font> {
    options: ColorCycleViewerOptions<'font>,
    file_index: usize,
    current_time: Option<u64>,
    time_speed: u64,
    was_resized: bool,
    was_moved: bool,
    x: i32,
    y: i32,

    #[allow(unused)]
    sdl: sdl2::Sdl,
    font: Option<sdl2::ttf::Font<'font, 'static>>,
    font_size: u16,
    #[allow(unused)]
    video: sdl2::VideoSubsystem,
    canvas: sdl2::render::WindowCanvas,
    event_pump: sdl2::EventPump,
}

const MESSAGE_DISPLAY_DURATION: Duration = Duration::from_secs(3);
const ERROR_MESSAGE_DISPLAY_DURATION: Duration = Duration::from_secs(1000 * 365 * 24 * 60 * 60);

impl<'font> ColorCycleViewer<'font> {
    pub fn new(options: ColorCycleViewerOptions<'font>) -> Result<ColorCycleViewer<'font>, error::Error> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let window = video
            .window(APP_NAME, 640, 480)
            .set_window_flags(if options.full_screen {
                SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32
            } else { 0 })
            .position_centered()
            .resizable()
            .build()?;
        let event_pump = sdl.event_pump()?;

        sdl.mouse().show_cursor(false);

        let canvas = window.into_canvas()
            .accelerated()
            .present_vsync()
            .build()?;

        Ok(ColorCycleViewer {
            options,
            current_time: None,
            time_speed: 1,
            file_index: 0,
            x: 0,
            y: 0,

            was_resized: false,
            was_moved: false,
            sdl,
            font: None,
            font_size: 0,
            video,
            canvas,
            event_pump,
        })
    }

    pub fn run(&mut self) -> Result<(), error::Error> {
        self.canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        self.canvas.clear();
        self.canvas.present();

        loop {
            match self.show_image()? {
                Action::Goto(index) => {
                    self.file_index = index;
                }
                Action::Quit => {
                    return Ok(());
                }
                Action::OpenFile(filename) => {
                    self.options.paths.push(filename.into());
                    self.file_index = self.options.paths.len() - 1;
                }
            }
        }
    }

    fn show_image(&mut self) -> Result<Action, error::Error> {
        let path = &self.options.paths[self.file_index];

        let filename = path.file_name().map(|f| f.to_string_lossy()).unwrap_or_else(|| path.to_string_lossy());
        self.canvas.window_mut().set_title(&format!("{filename} - {APP_NAME}")).log_error("window.set_title()");

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut x_aspect = 1;
        let mut y_aspect = 1;

        let living_world: Result<LivingWorld, error::Error> = match ilbm::ILBM::read(&mut reader) {
            Ok(ilbm) => {
                let ilbm_x_aspect = ilbm.header().x_aspect();
                let ilbm_y_aspect = ilbm.header().y_aspect();
                if ilbm_x_aspect != 0 && ilbm_y_aspect != 0 && ilbm_x_aspect != ilbm_y_aspect {
                    if ilbm_x_aspect % ilbm_y_aspect == 0 {
                        x_aspect = ilbm_x_aspect / ilbm_y_aspect;
                    } else if ilbm_y_aspect % ilbm_x_aspect == 0 {
                        y_aspect = ilbm_y_aspect / ilbm_x_aspect;
                    } else {
                        x_aspect = ilbm_x_aspect;
                        y_aspect = ilbm_y_aspect;
                    }
                }
                //let viewport_mode = ilbm.camg().map(CAMG::viewport_mode).unwrap_or(0);
                //eprintln!("ILBM: file_type: {:?}, {:?}", ilbm.file_type(), ilbm.header());
                //eprintln!("colors: {}", ilbm.cmap().map_or(0, |cmap| cmap.colors().len()));
                //eprint!("viewport_mode: 0x{viewport_mode:x}");
                //for &(flag, name) in &[
                //    (CAMG::EHB, "EHB"),
                //    (CAMG::HAM, "HAM"),
                //    (CAMG::HIRES, "HIRES"),
                //    (CAMG::LACE, "LACE"),
                //] {
                //    if viewport_mode & flag != 0 {
                //        eprint!(" {name}");
                //    }
                //}
                //eprintln!();
                let res: Result<CycleImage, _> = ilbm.try_into();
                match res {
                    Ok(image) => Ok(image.into()),
                    Err(err) => Err(err.into())
                }
            }
            Err(err) => {
                if err.kind() != ilbm::ErrorKind::UnsupportedFileFormat {
                    Err(err.into())
                } else if let Err(err) = reader.seek(std::io::SeekFrom::Start(0)) {
                    Err(err.into())
                } else {
                    match serde_json::from_reader(&mut reader) {
                        Ok(image) => Ok(image),
                        Err(err) => Err(err.into())
                    }
                }
            }
        };
        drop(reader);

        let mut message = String::new();
        let mut message_end_ts = Instant::now();
        let mut living_world = match living_world {
            Ok(living_world) => {
                if living_world.base().width() == 0 || living_world.base().height() == 0 {
                    message_end_ts += ERROR_MESSAGE_DISPLAY_DURATION;
                    let _ = write!(message, " {filename}: image of size {} x {} ",
                        living_world.base().width(),
                        living_world.base().height());
                    x_aspect = 1;
                    y_aspect = 1;
                    CycleImage::new(None, IndexedImage::new(640, 480, Palette::default()), Box::new([])).into()
                } else {
                    if self.options.osd {
                        if let Some(name) = living_world.name() {
                            let _ = write!(message, " {name} ({filename}) ");
                        } else {
                            let _ = write!(message, " {filename} ");
                        }
                        message_end_ts += MESSAGE_DISPLAY_DURATION
                    }

                    living_world
                }
            },
            Err(err) => {
                message_end_ts += ERROR_MESSAGE_DISPLAY_DURATION;
                let _ = write!(message, " {filename}: {err} ");
                x_aspect = 1;
                y_aspect = 1;
                CycleImage::new(None, IndexedImage::new(640, 480, Palette::default()), Box::new([])).into()
            }
        };

        let cycle_image = living_world.base();
        let img_width  = cycle_image.width();
        let img_height = cycle_image.height();

        self.canvas.window_mut().set_title(&if let Some(name) = living_world.name() {
            format!("{name} ({filename}) - {img_width}x{img_height} - {APP_NAME}")
        } else {
            format!("{filename} - {img_width}x{img_height} - {APP_NAME}")
        }).log_error("window.set_title()");

        // TODO: implement full worlds demo support
        let mut blended_palette = cycle_image.palette().clone();
        let mut cycled_palette1 = blended_palette.clone();
        let mut cycled_palette2 = blended_palette.clone();

        let mut frame_duration = Duration::from_secs_f64(1.0 / (self.options.fps as f64));

        let fixed_width  = img_width  * x_aspect as u32;
        let fixed_height = img_height * y_aspect as u32;

        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator.create_texture(
            PixelFormatEnum::RGB24,
            sdl2::render::TextureAccess::Streaming,
            img_width, img_height
        )?;

        if !self.was_resized {
            if self.canvas.window().fullscreen_state() == FullscreenType::Off {
                // Guess if the window is approximately cnetered on the screen and
                // if yes, then re-center after resizing.
                let window = self.canvas.window_mut();
                let display_mode = self.video.current_display_mode(window.display_index()?)?;
                let (win_width, win_height) = window.size();
                let (win_x, win_y) = window.position();
                let expected_x = (display_mode.w - win_width  as i32) / 2;
                let expected_y = (display_mode.h - win_height as i32) / 2;
                let is_centered =
                    (expected_x - win_x).abs() <= display_mode.w / 20 &&
                    (expected_y - win_y).abs() <= display_mode.h / 20;

                window.set_size(fixed_width, fixed_height).log_error("window.set_size()");

                if is_centered {
                    window.set_position(WindowPos::Centered, WindowPos::Centered);
                }
            }
        }

        let mut message_texture = None;

        self.canvas.set_integer_scale(true).log_error("canvas.set_integer_scale(true)");

        let loop_start_ts = Instant::now();

        loop {
            let frame_start_ts = Instant::now();
            let mut time_of_day = if let Some(current_time) = self.current_time {
                current_time
            } else {
                get_time_of_day_msec(self.time_speed)
            };

            macro_rules! show_message {
                ($($args:expr),+) => {
                    if self.options.osd {
                        message_end_ts = frame_start_ts + MESSAGE_DISPLAY_DURATION;
                        message.clear();
                        message.push_str(" ");
                        let _ = write!(&mut message, $($args),+);
                        message.push_str(" ");
                        message_texture = None;
                    }
                };
            }

            // process input
            while let Some(event) = self.event_pump.poll_event() {
                match event {
                    Event::Window { win_event, .. } => {
                        match win_event {
                            WindowEvent::Resized(_, _) => {
                                self.was_resized = true;
                            }
                            _ => {}
                        }
                    }
                    Event::Quit { .. } => {
                        return Ok(Action::Quit);
                    }
                    Event::KeyDown { keycode, keymod, repeat, .. } => {
                        if let Some(keycode) = keycode {
                            match keycode {
                                Keycode::Q => {
                                    // quit
                                    return Ok(Action::Quit);
                                }
                                Keycode::ESCAPE => {
                                    let window = self.canvas.window_mut();
                                    if window.fullscreen_state() == FullscreenType::Off {
                                        return Ok(Action::Quit);
                                    }
                                    window.set_fullscreen(FullscreenType::Off)?;
                                }
                                Keycode::B => {
                                    // toggle blend mode
                                    self.options.blend = !self.options.blend;

                                    show_message!("Blend Mode: {}", if self.options.blend { "Enabled" } else { "Disabled" });
                                }
                                Keycode::C => {
                                    // toggle cover/contain
                                    self.options.cover = !self.options.cover;

                                    if self.options.cover {
                                        show_message!("Zoom to cover");
                                    } else {
                                        show_message!("Zoom to contain");
                                    }
                                }
                                Keycode::O => {
                                    // toggle OSD
                                    if self.options.osd {
                                        show_message!("OSD: Disabled");
                                        self.options.osd = false;
                                    } else {
                                        self.options.osd = true;
                                        show_message!("OSD: Enabled");
                                    }
                                }
                                Keycode::PLUS | Keycode::KP_PLUS => {
                                    // increase FPS
                                    if self.options.fps < MAX_FPS {
                                        self.options.fps += 1;
                                        frame_duration = Duration::from_secs_f64(1.0 / self.options.fps as f64);

                                        show_message!("FPS: {}", self.options.fps);
                                    }
                                }
                                Keycode::MINUS | Keycode::KP_MINUS => {
                                    // decrease FPS
                                    if self.options.fps > 1 {
                                        self.options.fps -= 1;
                                        frame_duration = Duration::from_secs_f64(1.0 / self.options.fps as f64);

                                        show_message!("FPS: {}", self.options.fps);
                                    }
                                }
                                Keycode::N => {
                                    // next file
                                    let new_index = self.file_index + 1;
                                    if new_index >= self.options.paths.len() {
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
                                    let time_step = if keymod.bits() & SHIFT != 0 { SMALL_TIME_STEP } else { TIME_STEP };
                                    let rem = time_of_day % time_step;
                                    let new_time = time_of_day - rem;
                                    if new_time == time_of_day {
                                        if new_time < time_step {
                                            time_of_day = DAY_DURATION - time_step;
                                        } else {
                                            time_of_day = new_time - time_step;
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
                                    let time_step = if keymod.bits() & SHIFT != 0 { SMALL_TIME_STEP } else { TIME_STEP };
                                    let rem = time_of_day % time_step;
                                    let new_time = time_of_day - rem + time_step;
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
                                    self.time_speed = 1;
                                    self.current_time = None;
                                    time_of_day = get_time_of_day_msec(self.time_speed);
                                    let (hours, mins) = get_hours_mins(time_of_day);
                                    show_message!("{hours}:{mins:02}");
                                }
                                Keycode::F | Keycode::F11 => {
                                    // toggle fullscreen
                                    if !repeat {
                                        let window = self.canvas.window_mut();
                                        let value = match window.fullscreen_state() {
                                            FullscreenType::Desktop | FullscreenType::True => FullscreenType::Off,
                                            FullscreenType::Off => FullscreenType::Desktop,
                                        };
                                        window.set_fullscreen(value).log_error("window.set_fullscreen()");
                                    }
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
                                        self.current_time = Some(time_of_day);
                                        show_message!("Fast Forward: OFF");
                                    }
                                }
                                Keycode::I => {
                                    // ILBM column swap
                                    living_world.column_swap();
                                }
                                Keycode::UP => {
                                    self.move_y(get_move_amount(keymod) * y_aspect as i32);
                                }
                                Keycode::DOWN => {
                                    self.move_y(-get_move_amount(keymod) * y_aspect as i32);
                                }
                                Keycode::LEFT => {
                                    self.move_x(get_move_amount(keymod) * x_aspect as i32);
                                }
                                Keycode::RIGHT => {
                                    self.move_x(-get_move_amount(keymod) * x_aspect as i32);
                                }
                                Keycode::HOME => {
                                    if self.options.cover {
                                        if keymod.bits() & CTRL != 0 {
                                            self.y = 0;
                                        } else {
                                            self.x = 0;
                                        }
                                        self.was_moved = true;
                                    }
                                }
                                Keycode::END => {
                                    if self.options.cover {
                                        if keymod.bits() & CTRL != 0 {
                                            self.y = i32::MIN;
                                        } else {
                                            self.x = i32::MIN;
                                        }
                                        self.was_moved = true;
                                    }
                                }
                                Keycode::KP_0 | Keycode::NUM_0 => {
                                    return Ok(Action::Goto(self.options.paths.len() - 1));
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
                                        if index as usize >= self.options.paths.len() {
                                            show_message!("Only {} files opened!", self.options.paths.len());
                                        } else {
                                            return Ok(Action::Goto(index as usize));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Event::DropFile { filename, .. } => {
                        return Ok(Action::OpenFile(filename));
                    }
                    _ => {}
                }
            }

            // render frame
            let blend_cycle = (frame_start_ts - loop_start_ts).as_secs_f64();
            let palette;
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

                cycled_palette1.apply_cycles_from(palette1.palette(), palette1.cycles(), blend_cycle, self.options.blend);
                cycled_palette2.apply_cycles_from(palette2.palette(), palette2.cycles(), blend_cycle, self.options.blend);

                crate::palette::blend(&cycled_palette1, &cycled_palette2, blend_palettes, &mut blended_palette);

                palette = &blended_palette;
            } else {
                cycled_palette1.apply_cycles_from(&blended_palette, living_world.base().cycles(), blend_cycle, self.options.blend);
                palette = &cycled_palette1;
            }

            texture.with_lock(None, |pixels, pitch| {
                let indexed_image = living_world.base().indexed_image();
                for y in 0..img_height {
                    let y_offset = y as usize * pitch;
                    for x in 0..img_width {
                        let index = indexed_image.get_index(x, y);
                        let pixel_offset = y_offset + 3 * x as usize;
                        let Rgb([r, g, b]) = palette[index];
                        pixels[pixel_offset    ] = r;
                        pixels[pixel_offset + 1] = g;
                        pixels[pixel_offset + 2] = b;
                    }
                }
            })?;

            self.canvas.clear();
            let (canvas_width, canvas_height) = self.canvas.output_size()?;

            let mut draw_width;
            let mut draw_height;
            let draw_x;
            let draw_y;

            draw_width = canvas_width;
            draw_height = fixed_height * canvas_width / fixed_width;

            if self.options.cover {
                if draw_height < canvas_height {
                    draw_width = fixed_width * canvas_height / fixed_height;
                    draw_height = canvas_height;
                }

                let min_x = if draw_width > canvas_width {
                    -((draw_width - canvas_width) as i32)
                } else { 0 };

                let min_y = if draw_height > canvas_height {
                    -((draw_height - canvas_height) as i32)
                } else { 0 };

                if self.was_moved {
                    let img_min_x = min_x * fixed_width as i32 / draw_width as i32;
                    let img_min_y = min_y * fixed_height as i32 / draw_height as i32;

                    self.x = self.x.clamp(img_min_x, 0);
                    self.y = self.y.clamp(img_min_y, 0);

                    draw_x = self.x * draw_width as i32 / fixed_width as i32;
                    draw_y = self.y * draw_height as i32 / fixed_height as i32;
                } else {
                    draw_x = min_x / 2;
                    draw_y = min_y / 2;

                    self.x = draw_x * fixed_width as i32 / draw_width as i32;
                    self.y = draw_y * fixed_height as i32 / draw_height as i32;
                }
            } else {
                if draw_height > canvas_height {
                    draw_width = fixed_width * canvas_height / fixed_height;
                    draw_height = canvas_height;
                }

                draw_x = if draw_width < canvas_width {
                    ((canvas_width - draw_width) / 2) as i32
                } else { 0 };

                draw_y = if draw_height < canvas_height {
                    ((canvas_height - draw_height) / 2) as i32
                } else { 0 };
            }

            self.canvas.copy(&texture, None, Rect::new(draw_x, draw_y, draw_width, draw_height))?;

            if self.time_speed != 1 && message.is_empty() {
                let (hours, mins) = get_hours_mins(time_of_day);
                show_message!("{hours}:{mins:02}");
            }

            if message_end_ts >= frame_start_ts {
                // draw OSD message
                let new_font_size = (canvas_height / 30) as u16;
                if new_font_size != self.font_size {
                    self.font = None;
                    message_texture = None;
                }

                let texture = if let Some(texture) = &message_texture {
                    texture
                } else {
                    let font = if let Some(font) = &self.font {
                        font
                    } else {
                        self.font = Some(self.options.ttf.load_font_from_rwops(
                            RWops::from_bytes(HACK_FONT)?,
                            new_font_size)?);
                        self.font_size = new_font_size;
                        self.font.as_ref().unwrap()
                    };

                    let surface = font.render(&message)
                        .shaded(Color::RGB(255, 255, 255), Color::RGB(0, 0, 0))?;

                    message_texture = Some(texture_creator
                        .create_texture_from_surface(surface)?);

                    message_texture.as_ref().unwrap()
                };

                let TextureQuery { width, height, .. } = texture.query();

                self.canvas.copy(&texture, None, Rect::new(
                    (canvas_width as i32 - width as i32) / 2,
                    canvas_height as i32 - height as i32 - new_font_size as i32,
                    width, height))?;
            }

            self.canvas.present();

            // sleep for rest of frame
            let elapsed = frame_start_ts.elapsed();
            if frame_duration > elapsed && !interruptable_sleep(frame_duration - elapsed) {
                return Ok(Action::Quit);
            }
        }
    }

    fn move_x(&mut self, amount: i32) {
        if self.options.cover {
            if amount > 0 {
                if self.x > i32::MAX - amount {
                    self.x = i32::MAX;
                } else {
                    self.x += amount;
                }
            } else {
                if self.x < i32::MIN - amount {
                    self.x = i32::MIN;
                } else {
                    self.x += amount;
                }
            }
            self.was_moved = true;
        }
    }

    fn move_y(&mut self, amount: i32) {
        if self.options.cover {
            if amount > 0 {
                if self.y > i32::MAX - amount {
                    self.y = i32::MAX;
                } else {
                    self.y += amount;
                }
            } else {
                if self.y < i32::MIN - amount {
                    self.y = i32::MIN;
                } else {
                    self.y += amount;
                }
            }
            self.was_moved = true;
        }
    }
}

// const ALT: u16 = Mod::LALTMOD.bits() | Mod::RALTMOD.bits();
const SHIFT: u16 = Mod::LSHIFTMOD.bits() | Mod::RSHIFTMOD.bits();
const CTRL: u16 = Mod::LCTRLMOD.bits() | Mod::RCTRLMOD.bits();

#[inline]
fn get_move_amount(keymod: Mod) -> i32 {
    let keymod = keymod.bits();
    if keymod & CTRL != 0 {
        10
    } else {
        1
    }
}

enum Action {
    Goto(usize),
    Quit,
    OpenFile(String),
}

fn get_time_of_day_msec(time_speed: u64) -> u64 {
    #[cfg(not(windows))]
    unsafe {
        let mut tod = MaybeUninit::<libc::timespec>::zeroed();
        if libc::clock_gettime(libc::CLOCK_REALTIME, tod.as_mut_ptr()) != 0 {
            return 0;
        }
        let tod = tod.assume_init_ref();
        let mut tm = MaybeUninit::<libc::tm>::zeroed();
        if libc::localtime_r(&tod.tv_sec, tm.as_mut_ptr()).is_null() {
            return 0;
        }
        let tm = tm.assume_init_ref();
        let mut now = Duration::new(tod.tv_sec as u64, tod.tv_nsec as u32);

        if tm.tm_gmtoff > 0 {
            now += Duration::from_secs(tm.tm_gmtoff as u64);
        } else {
            now -= Duration::from_secs((-tm.tm_gmtoff) as u64);
        }

        ((now.as_millis() * time_speed as u128) % DAY_DURATION as u128) as u64
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
            eprintln!("ERROR: {msg}: {}", err);
        }
    }

    #[inline]
    fn log_info(&self, msg: &str) {
        if let Err(err) = self {
            println!("INFO: {msg}: {}", err);
        }
    }

    #[inline]
    fn log_warning(&self, msg: &str) {
        if let Err(err) = self {
            println!("WARNING: {msg}: {}", err);
        }
    }
}
