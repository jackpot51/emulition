extern crate rustc_serialize;
extern crate sdl2;
extern crate sdl2_image;
extern crate sdl2_ttf;
extern crate toml;

use std::cmp::min;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sdl2::controller::{Axis, Button};
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::mouse::Mouse;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use cursor::Cursor;
use emulator::{Emulator, EmulatorConfig};
use font::Font;
use rom::{Progress, Rom};
use texture::NormalTexture;

pub mod cursor;
pub mod doperoms;
pub mod emulator;
pub mod font;
pub mod rom;
pub mod texture;

pub fn ls(path: &str) -> Vec<String> {
    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(path) {
        for entry_result in read_dir {
            if let Ok(entry) = entry_result {
                if let Some(path) = entry.path().to_str() {
                    entries.push(path.to_string());
                }
            }
        }
    }

    entries.sort();

    entries
}

#[derive(Clone, PartialEq)]
enum View {
    Rom(String, usize),
    Emulator(String, bool),
    Overview
}

fn main(){
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();
    let _ttf_context = sdl2_ttf::init();

    let window = video_subsystem.window("emulition", 1024, 768)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    let mut controllers = Vec::new();
    for id in 0 .. controller_subsystem.num_joysticks().unwrap() {
        if controller_subsystem.is_game_controller(id) {
            controllers.push(controller_subsystem.open(id).unwrap());
        }
    }

    let mut cursor = Cursor::new(&renderer, "res/cursor.png");

    let font = Font::new("res/DroidSans.ttf", 24);

    let mut emulators = BTreeMap::new();

    if let Ok(mut file) = File::open("config.toml") {
        let mut toml = String::new();
        if let Ok(_) = file.read_to_string(&mut toml) {
            if let Some(parsed) = toml::Parser::new(&toml).parse() {
                for (key, value) in parsed {
                    if let Some(config) = toml::decode::<EmulatorConfig>(value) {
                        emulators.insert(key, Emulator::new(&renderer, &font, config));
                    }
                }
            }
        }
    }

    let playing_rom = Arc::new(Mutex::new(None));

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut view = View::Overview;
    let mut offset = 0;
    'running: loop {
        let mut forward = false;
        let mut backward = false;
        let mut scroll = 0.0;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { scancode: Some(Scancode::Escape), .. } => break 'running,
                Event::KeyDown { scancode: Some(Scancode::Return), .. } => forward = true,
                Event::KeyDown { scancode: Some(Scancode::Backspace), .. } => backward = true,
                Event::ControllerButtonDown { button: Button::A, .. } => forward = true,
                Event::ControllerButtonDown { button: Button::B, .. } => backward = true,
                Event::MouseButtonDown { mouse_btn: Mouse::Left, .. } => forward = true,
                Event::MouseButtonDown { mouse_btn: Mouse::Right, .. } => backward = true,
                Event::MouseMotion { x, y, .. } => cursor.set(&renderer, x as f32, y as f32),
                Event::MouseWheel { y, .. } => scroll += y as f32 * 64.0,
                _ => {}
            }
        }

        if event_pump.keyboard_state().is_scancode_pressed(Scancode::Left) {
            cursor.offset(&renderer, -8.0, 0.0);
        }
        if event_pump.keyboard_state().is_scancode_pressed(Scancode::Right) {
            cursor.offset(&renderer, 8.0, 0.0);
        }
        if event_pump.keyboard_state().is_scancode_pressed(Scancode::Up) {
            cursor.offset(&renderer, 0.0, -8.0);
        }
        if event_pump.keyboard_state().is_scancode_pressed(Scancode::Down) {
            cursor.offset(&renderer, 0.0, 8.0);
        }
        if event_pump.keyboard_state().is_scancode_pressed(Scancode::PageUp) {
            scroll += 32.0;
        }
        if event_pump.keyboard_state().is_scancode_pressed(Scancode::PageDown) {
            scroll -= 32.0;
        }

        for controller in controllers.iter() {
            let dx = controller.axis(Axis::LeftX) as f32 / 32768.0;
            let dy = controller.axis(Axis::LeftY) as f32 / 32768.0;
            if (dx.powi(2) + dy.powi(2)).sqrt() > 0.2 {
                cursor.offset(&renderer, dx * 16.0, dy * 16.0);
            }

            let dz = controller.axis(Axis::RightY) as f32 / 32768.0;
            if dz.abs() > 0.2 {
                scroll -= dz * 32.0;
            }

            if controller.button(Button::DPadLeft) {
                cursor.offset(&renderer, -8.0, 0.0);
            }
            if controller.button(Button::DPadRight) {
                cursor.offset(&renderer, 8.0, 0.0);
            }
            if controller.button(Button::DPadUp) {
                cursor.offset(&renderer, 0.0, -8.0);
            }
            if controller.button(Button::DPadDown) {
                cursor.offset(&renderer, 0.0, 8.0);
            }
        }

        offset += scroll as i32;

        let highlight_color = Color::RGB(224, 224, 224);
        renderer.set_draw_color(Color::RGB(255, 255, 255));
        renderer.clear();

        let mut x = 0;
        let mut y = 0;
        let width = renderer.output_size().unwrap().0 as i32;
        let height = renderer.output_size().unwrap().1 as i32;
        let mut s = min(width / 4, height / 3);

        let mut new_view = view.clone();
        match view {
            View::Rom(ref key, index) => {
                if let Some(emulator) = emulators.get(key) {
                    emulator.draw(&mut renderer, &font, x, y, s, s);
                    y += s;

                    if let Some(rom) = emulator.roms.get(index) {
                        x = s;
                        y = 0;
                        s = s * 3;

                        if cursor.inside(x, y, s, s) {
                            renderer.set_draw_color(highlight_color);
                            renderer.fill_rect(Rect::new(x, y, s as u32, s as u32).unwrap().unwrap());

                            if forward {
                                let can_run = playing_rom.lock().unwrap().is_none();
                                if can_run {
                                    let mut command = emulator.run(rom);

                                    println!("launching: {:?}", command);
                                    match command.spawn() {
                                        Ok(mut child) => {
                                            *playing_rom.lock().unwrap() = Some(rom.config.name.clone());

                                            let playing_rom_clone = playing_rom.clone();
                                            thread::spawn(move || {
                                                println!("exited: {:?}", child.wait());
                                                *playing_rom_clone.lock().unwrap() = None;
                                            });
                                        },
                                        Err(err) => {
                                            println!("error: {:?}", err);
                                        }
                                    }
                                } else {
                                    println!("emulator already running: {:?}", *playing_rom.lock().unwrap());
                                }
                            }
                        }

                        rom.draw(&mut renderer, &font, x, y, s, s);

                        x = s;
                        y = 0;

                        if backward {
                            new_view = View::Emulator(key.clone(), false);
                        }
                    } else {
                        new_view = View::Emulator(key.clone(), false);
                    }
                } else {
                    new_view = View::Overview
                }
            },
            View::Emulator(ref key, downloads) => {
                if let Some(mut emulator) = emulators.get_mut(key) {
                    emulator.draw(&mut renderer, &font, x, y, s, s);
                    y += s;

                    if cursor.inside(x, y, s, 32) {
                        renderer.set_draw_color(highlight_color);
                        renderer.fill_rect(Rect::new(x, y, s as u32, 32).unwrap().unwrap());
                        if forward {
                            new_view = View::Emulator(key.clone(), false);
                        }
                    }
                    let texture = NormalTexture::new(font.render(&renderer, &format!("Installed: {}", emulator.roms.len()), Color::RGB(0, 0, 0)));
                    texture.draw(&mut renderer, x + 8, y + 4);
                    y += 32;

                    if let Some(ref doperoms) = emulator.doperoms {
                        let text = match doperoms.progress() {
                            Progress::Connecting => "Internet: ...".to_string(),
                            Progress::InProgress(downloaded, total) => {
                                if total > 0 {
                                    let ratio = downloaded as f64 / total as f64;
                                    let pixels = (s as f64 * ratio) as u32;
                                    if pixels > 0 {
                                        renderer.set_draw_color(Color::RGB(0, 255, 0));
                                        renderer.fill_rect(Rect::new(x, y, pixels, 32).unwrap().unwrap());
                                    }
                                    format!("Internet: {:.1}%", ratio * 100.0)
                                } else {
                                    format!("Internet: ?%")
                                }
                            },
                            Progress::Error(error) => format!("Internet: {}", error),
                            Progress::Complete => "Internet: Complete".to_string()
                        };

                        let texture = NormalTexture::new(font.render(&renderer, &text, Color::RGB(0, 0, 0)));
                        texture.draw(&mut renderer, x + 8, y + 4);
                        y += 32;
                    } else{
                        if cursor.inside(x, y, s, 32) {
                            renderer.set_draw_color(highlight_color);
                            renderer.fill_rect(Rect::new(x, y, s as u32, 32).unwrap().unwrap());
                            if forward {
                                new_view = View::Emulator(key.clone(), true);
                            }
                        }
                        let texture = NormalTexture::new(font.render(&renderer, &format!("Internet: {}", emulator.downloads.len()), Color::RGB(0, 0, 0)));
                        texture.draw(&mut renderer, x + 8, y + 4);
                        y += 32;
                    }

                    x = s;
                    y = offset;
                    if downloads {
                        let mut download_option = None;
                        for rom in emulator.downloads.iter() {

                            if y + 32 >= 0 && y < height {
                                if cursor.inside(x, y, width - x, 32) {
                                    renderer.set_draw_color(highlight_color);
                                    renderer.fill_rect(Rect::new(x, y, (width - x) as u32, 32).unwrap().unwrap());
                                    if forward {
                                        download_option = Some(rom.clone());
                                    }
                                }

                                let texture = NormalTexture::new(font.render(&renderer, &format!("{}", rom.file) , Color::RGB(0, 0, 0)));
                                texture.draw(&mut renderer, x + 8, y + 4);
                            }
                            y += 32;
                            /*
                            if y + 96 >= 0 && y < height {
                                if cursor.inside(x, y, width - x, 96) {
                                    renderer.set_draw_color(highlight_color);
                                    renderer.fill_rect(Rect::new(x, y, (width - x) as u32, 96).unwrap().unwrap());
                                    if forward {
                                        download_option = Some(rom.clone());
                                    }
                                }

                                let texture = NormalTexture::new(font.render(&renderer, &format!("{}", rom.name) , Color::RGB(0, 0, 0)));
                                texture.draw(&mut renderer, x + 8, y + 4);

                                let texture = NormalTexture::new(font.render(&renderer, &format!("{}", rom.file) , Color::RGB(0, 0, 0)));
                                texture.draw(&mut renderer, x + 8 + 64, y + 4 + 32);

                                let texture = NormalTexture::new(font.render(&renderer, &format!("Flags: {:?}", rom.flags) , Color::RGB(0, 0, 0)));
                                texture.draw(&mut renderer, x + 8 + 64, y + 4 + 64);
                            }
                            y += 96;
                            */
                        }

                        if let Some(config) = download_option.take() {
                            let mut exists = false;
                            for rom in emulator.roms.iter() {
                                if rom.config.name == config.name {
                                    exists = true;
                                }
                            }

                            if ! exists {
                                let mut rom = Rom::new(&renderer, config);

                                {
                                    let mut image_path = PathBuf::from("roms");
                                    image_path.push(&key);
                                    image_path.push(&rom.config.name);
                                    image_path.push("image.jpg");
                                    if ! image_path.is_file() {
                                        rom.image_dl = Some(doperoms::Download::new(&rom.config.image, &image_path));
                                    }
                                    rom.config.image = format!("roms/{}/{}/image.jpg", key, rom.config.name);
                                }

                                {
                                    let mut rom_path = PathBuf::from("roms");
                                    rom_path.push(&key);
                                    rom_path.push(&rom.config.name);
                                    rom_path.push(&rom.config.file);
                                    if ! rom_path.is_file() {
                                        rom.doperoms = Some(doperoms::Download::rom(&emulator.config.doperoms, &rom.config.file, &rom_path));
                                    }
                                    rom.config.file = format!("roms/{}/{}/{}", key, rom.config.name, rom.config.file);
                                }

                                emulator.roms.push(rom);
                            } else {
                                println!("already downloaded {}", config.file);
                            }
                        }
                    } else {
                        for index in 0 .. emulator.roms.len() {
                            if let Some(rom) = emulator.roms.get(index) {
                                if y + s >= 0 && y < height {
                                    if cursor.inside(x, y, s, s) {
                                        renderer.set_draw_color(highlight_color);
                                        renderer.fill_rect(Rect::new(x, y, s as u32, s as u32).unwrap().unwrap());

                                        if forward {
                                            new_view = View::Rom(key.clone(), index);
                                        }
                                    }

                                    rom.draw(&mut renderer, &font, x, y, s, s);
                                }

                                x += s;

                                if x + s > renderer.output_size().unwrap().0 as i32 {
                                    x = s;
                                    y += s;
                                }
                            }
                        }
                    }

                    if backward {
                        new_view = View::Overview
                    }
                } else {
                    new_view = View::Overview
                }
            },
            View::Overview => {
                for (key, emulator) in emulators.iter() {
                    if cursor.inside(x, y, s, s) {
                        renderer.set_draw_color(highlight_color);
                        renderer.fill_rect(Rect::new(x, y, s as u32, s as u32).unwrap().unwrap());

                        if forward {
                            new_view = View::Emulator(key.clone(), false);
                        }
                    }

                    emulator.draw(&mut renderer, &font, x, y, s, s);

                    x += s;
                    if x + s > renderer.output_size().unwrap().0 as i32 {
                        x = 0;
                        y += s;
                    }
                }
            }
        };

        cursor.draw(&mut renderer);

        renderer.present();

        if new_view != view {
            offset = 0;
            view = new_view;
        } else {
            for (_, mut emulator) in emulators.iter_mut() {
                emulator.update(&renderer);
            }

            std::thread::sleep(Duration::from_millis(1000/60));
        }
    }
}
