use std::path::Path;

use sdl2::render::Renderer;

use sdl2_image::LoadTexture;

use texture::NormalTexture;

pub struct Cursor {
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
