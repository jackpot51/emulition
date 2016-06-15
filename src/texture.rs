use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};

pub struct NormalTexture {
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

pub struct CenteredTexture {
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

pub struct ScaledTexture {
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
