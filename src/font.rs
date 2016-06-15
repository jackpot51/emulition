use std::path::Path;

use sdl2::pixels::Color;
use sdl2::render::{Renderer, Texture};

use sdl2_ttf;

pub struct Font {
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
