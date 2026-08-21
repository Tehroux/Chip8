use std::{
    path::Path,
    time::{Duration, Instant},
};

use clap::Parser;
use minifb::{Key, Scale, Window, WindowOptions};

mod chip8;
mod error;

use chip8::{Chip8, HEIGHT, WIDTH};
use error::Chip8Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    rom: String,
}

fn main() -> Result<(), Chip8Error> {
    let args = Args::parse();

    let mut window = Window::new(
        "Chip 8",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X8,
            ..Default::default()
        },
    )
    .map_err(|_| Chip8Error::Window)?;

    let mut chip8 = Chip8::new();

    let path = Path::new(&args.rom);
    chip8.load(path)?;

    let mut delay_timer = Instant::now();
    let mut cpu_timer = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if cpu_timer.elapsed() > Duration::from_millis((1. / 500. * 1000_f64).round() as u64) {
            chip8.keys[0] = window.is_key_down(Key::Key0);
            chip8.keys[1] = window.is_key_down(Key::Key1);
            chip8.keys[2] = window.is_key_down(Key::Key2);
            chip8.keys[3] = window.is_key_down(Key::Key3);
            chip8.keys[4] = window.is_key_down(Key::Key4);
            chip8.keys[5] = window.is_key_down(Key::Key5);
            chip8.keys[6] = window.is_key_down(Key::Key6);
            chip8.keys[7] = window.is_key_down(Key::Key7);
            chip8.keys[8] = window.is_key_down(Key::Key8);
            chip8.keys[9] = window.is_key_down(Key::Key9);
            chip8.keys[10] = window.is_key_down(Key::A);
            chip8.keys[11] = window.is_key_down(Key::B);
            chip8.keys[12] = window.is_key_down(Key::C);
            chip8.keys[13] = window.is_key_down(Key::D);
            chip8.keys[14] = window.is_key_down(Key::E);
            chip8.keys[15] = window.is_key_down(Key::F);

            chip8.run();
            cpu_timer = Instant::now();
        }

        if delay_timer.elapsed() > Duration::from_millis(16) {
            chip8.dec_delay();
            window
                .update_with_buffer(chip8.get_buffer(), WIDTH, HEIGHT)
                .map_err(|_| Chip8Error::ScreenBuffer)?;
            chip8.set_vblank();
            delay_timer = Instant::now();
        }
    }

    Ok(())
}
