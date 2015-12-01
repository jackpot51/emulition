extern crate rustc_serialize;
extern crate sdl2;
extern crate sdl2_image;
extern crate sdl2_ttf;
extern crate toml;

use std::cmp::min;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use sdl2::controller::{Axis, Button};
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::mouse::Mouse;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};

use sdl2_image::LoadTexture;

pub mod doperoms;

struct NormalTexture {
    texture: Texture
}

impl NormalTexture {
    pub fn new(texture: Texture) -> NormalTexture {
        NormalTexture {
            texture: texture
        }
    }

    pub fn draw(&self, renderer: &mut Renderer, x: i32, y: i32) {
        let query = self.texture.query();
        renderer.copy(&self.texture, None, Rect::new(x, y, query.width, query.height).unwrap());
    }
}

struct CenteredTexture {
    texture: Texture
}

impl CenteredTexture {
    pub fn new(texture: Texture) -> CenteredTexture {
        CenteredTexture {
            texture: texture
        }
    }

    pub fn draw(&self, renderer: &mut Renderer, x: i32, y: i32, w: i32, h: i32) {
        let query = self.texture.query();
        let w2 = query.width as i32;
        let h2 = query.height as i32;
        let x2 = x + (w - w2)/2;
        let y2 = y + (h - h2)/2;
        if h2 > 0 && w2 > 0 {
            renderer.copy(&self.texture, None, Rect::new(x2, y2, w2 as u32, h2 as u32).unwrap());
        }
    }
}

struct ScaledTexture {
    texture: Texture
}

impl ScaledTexture {
    pub fn new(texture: Texture) -> ScaledTexture {
        ScaledTexture {
            texture: texture
        }
    }

    pub fn draw(&self, renderer: &mut Renderer, x: i32, y: i32, w: i32, h: i32) {
        let query = self.texture.query();
        let aspect = query.width as f32 / query.height as f32;
        let w2 = (aspect * h as f32).min(w as f32) as i32;
        let h2 = (w as f32 / aspect).min(h as f32) as i32;
        let x2 = x + (w - w2)/2;
        let y2 = y + (h - h2)/2;
        if h2 > 0 && w2 > 0 {
            renderer.copy(&self.texture, None, Rect::new(x2, y2, w2 as u32, h2 as u32).unwrap());
        }
    }
}


struct Cursor {
    pub x: f32,
    pub y: f32,
    texture: NormalTexture,
}

impl Cursor {
    pub fn new(renderer: &Renderer, image: &str) -> Cursor {
        Cursor {
            x: 0.0,
            y: 0.0,
            texture: NormalTexture::new(renderer.load_texture(&Path::new(image)).unwrap()),
        }
    }

    pub fn set(&mut self, renderer: &Renderer, x: f32, y: f32) {
        self.x = x.max(0.0).min(renderer.output_size().unwrap().0 as f32);
        self.y = y.max(0.0).min(renderer.output_size().unwrap().1 as f32);
    }

    pub fn offset(&mut self, renderer: &Renderer, dx: f32, dy: f32) {
        let x = self.x + dx;
        let y = self.y + dy;
        self.set(renderer, x, y);
    }

    pub fn inside(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        self.x >= x as f32 && self.x < (x as f32 + w as f32)
        && self.y >= y as f32 && self.y < (y as f32 + h as f32)
    }

    pub fn draw(&self, renderer: &mut Renderer) {
        self.texture.draw(renderer, (self.x - 26.0) as i32, (self.y - 4.0) as i32);
    }
}

struct Font {
    font: sdl2_ttf::Font
}

impl Font {
    pub fn new(font: &str, size: i32) -> Font {
        Font {
            font: sdl2_ttf::Font::from_file(&Path::new(font), size).unwrap()
        }
    }

    pub fn render(&self, renderer: &Renderer, text: &str, color: Color) -> Texture {
        let surface = self.font.render(text, sdl2_ttf::blended(color)).unwrap();
        return renderer.create_texture_from_surface(&surface).unwrap();
    }
}

#[derive(Clone)]
pub enum Progress {
    Connecting,
    InProgress(u64, u64),
    Error(String),
    Complete,
}

#[derive(Clone, Debug, Default)]
pub struct RomConfig {
    pub name: String,
    pub file: String,
    pub image: String,
}

struct Rom {
    image: Option<ScaledTexture>,
    doperoms: Option<doperoms::Download>,
    config: RomConfig,
}

impl Rom {
    pub fn new(renderer: &Renderer, font: &Font, config: RomConfig) -> Rom {
        Rom {
            image: if let Some(texture) = renderer.load_texture(&Path::new(&config.image)).ok() {
                Some(ScaledTexture::new(texture))
            } else {
                None
            },
            doperoms: None,
            config: config
        }
    }

    pub fn draw(&self, renderer: &mut Renderer, font: &Font, x: i32, y: i32, w: i32, h: i32) {
        let text = if let Some(ref doperoms) = self.doperoms {
            match doperoms.progress() {
                Progress::Connecting => format!("{}: ...", self.config.name),
                Progress::InProgress(downloaded, total) => {
                    if total > 0 {
                        let ratio = downloaded as f64 / total as f64;
                        let pixels = (w as f64 * ratio) as u32;
                        if pixels > 0 {
                            renderer.fill_rect(Rect::new(x, y, pixels, 32).unwrap().unwrap());
                        }
                        format!("{}: {:.1}%", self.config.name, ratio * 100.0)
                    } else {
                        format!("{}: ?%", self.config.name)
                    }
                },
                Progress::Error(error) => format!("{}: {}", self.config.name, error),
                Progress::Complete => format!("{}: Complete", self.config.name)
            }
        } else {
            self.config.name.clone()
        };

        let texture = CenteredTexture::new(font.render(&renderer, &text, Color::RGB(0, 0, 0)));
        texture.draw(renderer, x + 8, y + 4, w - 16, 24);

        if let Some(ref image) = self.image {
            image.draw(renderer, x + 8, y + 8 + 32, w - 16, h - 32 - 16);
        }
    }
}

#[derive(RustcDecodable)]
struct EmulatorConfig {
    pub name: String,
    pub image: String,
    pub roms: String,
    pub program: String,
    pub args: Vec<String>,
    pub doperoms: String,
}

struct Emulator {
    name: CenteredTexture,
    image: ScaledTexture,
    roms: Vec<Rom>,
    doperoms: Option<doperoms::List>,
    downloads: Vec<RomConfig>,
    config: EmulatorConfig
}

impl Emulator {
    pub fn new(renderer: &Renderer, font: &Font, config: EmulatorConfig) -> Emulator {
        let mut roms = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&config.roms) {
            for entry_result in read_dir {
                if let Ok(entry) = entry_result {
                    if let Some(path) = entry.path().to_str() {
                        roms.push(Rom::new(renderer, font, RomConfig {
                            name: path.replace(&config.roms, "").trim_matches('/').to_string(),
                            file: path.to_string() + "/rom.bin",
                            image: path.to_string() + "/image.jpg"
                        }))
                    }
                }
            }
        }

        roms.sort_by(|a, b| a.config.name.cmp(&b.config.name));

        Emulator {
            name: CenteredTexture::new(font.render(&renderer, &config.name, Color::RGB(0, 0, 0))),
            image: ScaledTexture::new(renderer.load_texture(&Path::new(&config.image)).unwrap()),
            roms: roms,
            doperoms: Some(doperoms::List::new(&config.doperoms)),
            downloads: Vec::new(),
            config: config
        }
    }

    pub fn draw(&self, renderer: &mut Renderer, font: &Font, x: i32, y: i32, w: i32, h: i32) {
        self.name.draw(renderer, x + 8, y + 4, w - 16, 24);
        self.image.draw(renderer, x + 8, y + 8 + 32, w - 16, h - 32 - 16);
    }

    pub fn run(&self, rom: &Rom){
        let mut command = Command::new(&self.config.program);
        for arg in self.config.args.iter() {
            if arg == "%r" {
                command.arg(&rom.config.file);
            }else{
                command.arg(arg);
            }
        }

        println!("status: {}", command.status().unwrap());
    }

    pub fn update(&mut self) {
        let take_doperoms = if let Some(ref doperoms) = self.doperoms {
            match doperoms.progress() {
                Progress::Complete => true,
                _ => false
            }
        } else {
            false
        };

        if take_doperoms {
            if let Some(doperoms) = self.doperoms.take() {
                for config in doperoms.result() {
                    self.downloads.push(config);
                }
            }
        }
    }
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

    if let Ok(mut file) = File::open("res/config.toml") {
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

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut view = View::Overview;
    let mut offset = 0;
    'running: loop {
        let mut forward = false;
        let mut backward = false;
        let mut scroll = 0;

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
                Event::MouseWheel { y, .. } => scroll += y,
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
            scroll -= 1;
        }
        if event_pump.keyboard_state().is_scancode_pressed(Scancode::PageDown) {
            scroll += 1;
        }

        for controller in controllers.iter() {
            let dx = controller.axis(Axis::LeftX) as f32 / 32768.0;
            let dy = controller.axis(Axis::LeftY) as f32 / 32768.0;
            if (dx.powi(2) + dy.powi(2)).sqrt() > 0.2 {
                cursor.offset(&renderer, dx * 8.0, dy * 8.0);
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

        offset += scroll * 32;

        renderer.set_draw_color(Color::RGB(255, 255, 255));
        renderer.clear();

        renderer.set_draw_color(Color::RGB(192, 192, 192));

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
                            renderer.fill_rect(Rect::new(x, y, s as u32, s as u32).unwrap().unwrap());

                            if forward {
                                emulator.run(rom);
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
                                    renderer.fill_rect(Rect::new(x, y, (width - x) as u32, 32).unwrap().unwrap());
                                    if forward {
                                        download_option = Some(rom.clone());
                                    }
                                }
                                let texture = NormalTexture::new(font.render(&renderer, &rom.name, Color::RGB(0, 0, 0)));
                                texture.draw(&mut renderer, x + 8, y + 4);
                            }
                            y += 32;
                        }

                        if let Some(mut config) = download_option.take() {
                            {
                                let mut image_path = PathBuf::from("roms");
                                image_path.push(&key);
                                image_path.push(&config.name);
                                image_path.push("image.jpg");
                                doperoms::Download::new(&config.image, &image_path).result();
                                if let Some(image) = image_path.to_str() {
                                    config.image = image.to_string();
                                }
                            }

                            let mut rom = Rom::new(&renderer, &font, config);
                            let mut rom_path = PathBuf::from("roms");
                            rom_path.push(&key);
                            rom_path.push(&rom.config.name);
                            rom_path.push(&rom.config.file);
                            rom.doperoms = Some(doperoms::Download::rom(&emulator.config.doperoms, &rom.config.file, &rom_path));
                            emulator.roms.push(rom);
                        }
                    } else {
                        for index in 0 .. emulator.roms.len() {
                            if let Some(rom) = emulator.roms.get(index) {
                                if y + s >= 0 && y < height {
                                    if cursor.inside(x, y, s, s) {
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
            view = new_view;
        } else {
            for (_, mut emulator) in emulators.iter_mut() {
                emulator.update();
            }

            std::thread::sleep(Duration::from_millis(1000/60));
        }
    }
}
