use std::path::Path;
use std::process::Command;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Renderer;

use sdl2_image::LoadTexture;

use doperoms;
use font::Font;
use ls;
use texture::{CenteredTexture, ScaledTexture};

#[derive(Clone)]
pub enum Progress {
    Connecting,
    InProgress(u64, u64),
    Error(String),
    Complete,
}

#[derive(Clone, Debug)]
pub enum RomFlags {
    Alternate,
    Bad,
    Cracked,
    Fix,
    Good,
    Hack,
    OverDump,
    PublicDomain,
    Trainer,
}

#[derive(Clone, Debug, Default)]
pub struct RomConfig {
    pub name: String,
    pub file: String,
    pub image: String,
    pub flags: Vec<RomFlags>,
}

pub struct Rom {
    image: Option<ScaledTexture>,
    pub image_dl: Option<doperoms::Download>,
    pub doperoms: Option<doperoms::Download>,
    pub config: RomConfig,
}

impl Rom {
    pub fn new(renderer: &Renderer, config: RomConfig) -> Rom {
        Rom {
            image: if let Some(texture) = renderer.load_texture(&Path::new(&config.image)).ok() {
                Some(ScaledTexture::new(texture))
            } else {
                None
            },
            image_dl: None,
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
                            renderer.set_draw_color(Color::RGB(0, 255, 0));
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
            format!("{}", self.config.name)
        };

        let texture = CenteredTexture::new(font.render(&renderer, &text, Color::RGB(0, 0, 0)));
        texture.draw(renderer, x + 8, y + 4, w - 16, 24);

        if let Some(ref image) = self.image {
            image.draw(renderer, x + 8, y + 8 + 32, w - 16, h - 32 - 16);
        }
    }

    pub fn update(&mut self, renderer: &Renderer){
        let take_image_dl = if let Some(ref image_dl) = self.image_dl {
            match image_dl.progress() {
                Progress::Complete => true,
                _ => false
            }
        } else {
            false
        };

        if take_image_dl {
            if let Some(image_dl) = self.image_dl.take() {
                if let Some(texture) = renderer.load_texture(&Path::new(&self.config.image)).ok() {
                    self.image = Some(ScaledTexture::new(texture));
                }
            }
        }

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
                let mut dir = String::new();
                if let Some(dir_path) = Path::new(&self.config.file).parent() {
                    if let Some(dir_str) = dir_path.to_str() {
                        dir = dir_str.to_string();
                    }
                }

                match Command::new("7z").arg("x").arg(&format!("-o{}", dir)).arg(&self.config.file).status() {
                    Ok(status) => {
                        println!("7z: {}", status);

                        if status.success() {
                            for file in ls(&dir) {
                                if file.ends_with(".jpg") || file.ends_with(".7z") || file.ends_with(".zip") {
                                } else {
                                    self.config.file = file;
                                }
                            }
                        }
                    },
                    Err(err) => println!("7z: {:?}", err)
                }
            }
        }
    }
}
