extern crate sdl2;
extern crate sdl2_image;

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
    texture: Texture,
    x: f32,
    y: f32,
}

impl Cursor {
    pub fn new(renderer: &Renderer) -> Cursor {
        Cursor {
            texture: renderer.load_texture(&Path::new("res/cursor.png")).unwrap(),
            x: 0.0,
            y: 0.0,
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

fn main(){
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();

    let window = video_subsystem.window("emulition", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    let mut cursor = Cursor::new(&renderer);

    let mut controllers = Vec::new();
    for id in 0 .. controller_subsystem.num_joysticks().unwrap() {
        if controller_subsystem.is_game_controller(id) {
            controllers.push(controller_subsystem.open(id).unwrap());
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

        cursor.draw(&mut renderer);

        renderer.present();

        std::thread::sleep(Duration::from_millis(1000/60));
    }
}
