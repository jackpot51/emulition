use std::path::Path;
use std::process::Command;

use sdl2::pixels::Color;
use sdl2::render::Renderer;

use sdl2_image::LoadTexture;

use doperoms;
use font::Font;
use rom::{Progress, Rom, RomConfig};
use ls;
use texture::{CenteredTexture, ScaledTexture};

#[derive(RustcDecodable)]
pub struct EmulatorConfig {
    pub name: String,
    pub image: String,
    pub roms: String,
    pub program: String,
    pub args: Vec<String>,
    pub doperoms: String,
}

pub struct Emulator {
    name: CenteredTexture,
    image: ScaledTexture,
    pub roms: Vec<Rom>,
    pub doperoms: Option<doperoms::List>,
    pub downloads: Vec<RomConfig>,
    pub config: EmulatorConfig
}

impl Emulator {
    pub fn new(renderer: &Renderer, font: &Font, config: EmulatorConfig) -> Emulator {
        let mut roms = Vec::new();
        for path in ls(&config.roms) {
            let mut rom = String::new();
            for file in ls(&path) {
                if file.ends_with(".jpg") || file.ends_with(".7z") || file.ends_with(".zip") {
                } else {
                    rom = file;
                }
            }

            roms.push(Rom::new(renderer, RomConfig {
                name: path.replace(&config.roms, "").trim_matches('/').to_string(),
                file: rom,
                image: path.to_string() + "/image.jpg",
                flags: Vec::new(),
            }));
        }

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

    pub fn run(&self, rom: &Rom) -> Command {
        let mut command = Command::new(&self.config.program);
        for arg in self.config.args.iter() {
            if arg == "%r" {
                command.arg(&rom.config.file);
            }else{
                command.arg(arg);
            }
        }

        command
    }

    pub fn update(&mut self, renderer: &Renderer) {
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

        for mut rom in self.roms.iter_mut() {
            rom.update(renderer);
        }
    }
}
