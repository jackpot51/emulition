extern crate rustc_serialize;
extern crate sdl2;
extern crate sdl2_image;
extern crate sdl2_ttf;
extern crate toml;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use sdl2::controller::Axis;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};

use sdl2_image::LoadTexture;

struct Cursor {
    x: f32,
    y: f32,
    texture: Texture,
}

impl Cursor {
    pub fn new(renderer: &Renderer, image: &str) -> Cursor {
        Cursor {
            x: 0.0,
            y: 0.0,
            texture: renderer.load_texture(&Path::new(image)).unwrap(),
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

    pub fn draw(&self, renderer: &mut Renderer) {
        renderer.copy(&self.texture, None, Rect::new((self.x - 26.0) as i32, (self.y - 4.0) as i32, 64, 64).unwrap());
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

#[derive(RustcDecodable)]
struct EmulatorConfig {
    pub name: String,
    pub command: String,
    pub image: String,
}

struct Emulator {
    name: Texture,
    command: String,
    image: Texture,
}

impl Emulator {
    pub fn new(renderer: &Renderer, font: &Font, config: EmulatorConfig) -> Emulator {
        Emulator {
            name: font.render(&renderer, &config.name, Color::RGBA(0, 0, 0, 255)),
            command: config.command,
            image: renderer.load_texture(&Path::new(&config.image)).unwrap(),
        }
    }

    pub fn draw(&self, renderer: &mut Renderer, x: i32, y: i32, w: u32, h: u32) {
        {
            let query = self.image.query();
            let aspect = query.width as f32 / query.height as f32;
    		let w2 = (aspect * h as f32).min(w as f32) as u32;
    		let h2 = (w as f32 / aspect).min(h as f32) as u32;
            let x2 = x + (w - w2) as i32/2;
            let y2 = y + (h - h2) as i32/2;
            renderer.copy(&self.image, None, Rect::new(x2, y2, w2, h2).unwrap());
        }

        {
            let query = self.name.query();
            let w2 = query.width;
            let h2 = query.height;
            let x2 = x + (w - w2) as i32/2;
            let y2 = y + h as i32;
            renderer.copy(&self.name, None, Rect::new(x2, y2, w2, h2).unwrap());
        }
    }
}

fn main(){
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();
    let _ttf_context = sdl2_ttf::init();

    let window = video_subsystem.window("emulition", 1024, 768)
        .position_centered()
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

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { scancode: Some(Scancode::Escape), .. } => break 'running,
                Event::MouseMotion { x, y, .. } => cursor.set(&renderer, x as f32, y as f32),
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

        for controller in controllers.iter() {
            let dx = controller.axis(Axis::LeftX) as f32 / 32768.0;
            let dy = controller.axis(Axis::LeftY) as f32 / 32768.0;
            if (dx.powi(2) + dy.powi(2)).sqrt() > 0.2 {
                cursor.offset(&renderer, dx * 8.0, dy * 8.0);
            }
        }

        renderer.set_draw_color(Color::RGB(255, 255, 255));
        renderer.clear();

        let mut x = 0;
        let mut y = 0;
        let s = renderer.output_size().unwrap().0 / 4;
        for (_, emulator) in emulators.iter() {
            emulator.draw(&mut renderer, x, y, s, s - 64);

            x += s as i32;
            if x + s as i32 > renderer.output_size().unwrap().0 as i32 {
                x = 0;
                y += s as i32;
            }
        }

        cursor.draw(&mut renderer);

        renderer.present();

        std::thread::sleep(Duration::from_millis(1000/60));
    }
}