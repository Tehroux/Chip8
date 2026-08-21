use std::{
    fs::File,
    io::Read,
    path::Path,
    time::{Duration, Instant},
};

use clap::Parser;
use minifb::{Key, Scale, Window, WindowOptions};
use rand::{RngExt, make_rng, rngs::SmallRng};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const MEMORY_SIZE: usize = 4096;
const STACK_SIZE: usize = 16;

const OFF_COLOR: u32 = 0xFF225522;
const ON_COLOR: u32 = 0xFF22AA22;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    rom: String,
}

#[derive(Debug)]
enum Chip8Error {
    Window,
    LoadingRom,
    ReadingRom,
    ScreenBuffer,
}

struct Chip8 {
    keys: Vec<bool>,
    vblank: bool,
    rng: SmallRng,
    screen: Vec<u32>,
    memory: Vec<u8>,
    stack: Vec<u16>,
    reg: Vec<u8>,
    i: u16,
    delay: u8,
    sound_timer: u8,
    pc: u16,
    sp: u8,
}

impl Default for Chip8 {
    fn default() -> Self {
        Self {
            vblank: false,
            keys: Vec::new(),
            rng: make_rng(),
            screen: Vec::new(),
            memory: Vec::new(),
            stack: Vec::new(),
            reg: Vec::new(),
            i: 0,
            delay: 0,
            sound_timer: 0,
            pc: 0,
            sp: 0,
        }
    }
}

impl Chip8 {
    fn new() -> Self {
        Self {
            keys: vec![false; 16],
            screen: vec![OFF_COLOR; WIDTH * HEIGHT],
            memory: vec![0; MEMORY_SIZE],
            stack: vec![0; STACK_SIZE],
            reg: vec![0;16],
            pc: 0x200,
            ..Default::default()
        }
    }

    fn get_reg(&self, n: u8) -> u8 {
        self.reg[n as usize]
    }

    fn set_reg(&mut self, n: u8, value: u8) {
        self.reg[n as usize] = value;
    }

    fn load(&mut self, rom_path: &Path) -> Result<(), Chip8Error> {
        let mut rom = File::open(rom_path).map_err(|i| {
            println!("{:?}, {:?}", i, rom_path);
            Chip8Error::LoadingRom
        })?;

        let r = rom.read(&mut self.memory[0x200..]).map_err(|i| {
            println!("{}", i);
            Chip8Error::ReadingRom
        })?;

        println!("read {r} bytes");

        Ok(())
    }

    fn dec_delay(&mut self) {
        if self.delay > 0 {
            self.delay -= 1;
        }
    }

    fn set_vblank(&mut self) {
        self.vblank = true;
    }

    fn run(&mut self) {
        let ih = self.memory[self.pc as usize];
        let il = self.memory[self.pc as usize + 1];
        let int = (ih & 0xF0) >> 4;
        let op1 = ih & 0x0F;
        let op2 = (il & 0xF0) >> 4;
        let op3 = il & 0x0F;

        match (int, op1, op2, op3) {
            (0x0, 0x0, 0xE, 0x0) => self.cls(),
            (0x0, 0x0, 0xE, 0xE) => self.ret(),
            (0x1, n1, n2, n3) => {
                self.jmp(n1, n2, n3);
                return;
            }
            (0x2, n1, n2, n3) => {
                self.call(n1, n2, n3);
                return;
            }
            (0x3, n1, n2, n3) => self.se(n1, n2, n3),
            (0x4, n1, n2, n3) => self.sne(n1, n2, n3),
            (0x5, n1, n2, 0x0) => self.se_reg(n1, n2),
            (0x6, n1, n2, n3) => self.ld(n1, n2, n3),
            (0x7, n1, n2, n3) => self.add(n1, n2, n3),
            (0x8, n1, n2, 0x0) => self.ld_reg(n1, n2),
            (0x8, n1, n2, 0x1) => self.or(n1, n2),
            (0x8, n1, n2, 0x2) => self.and(n1, n2),
            (0x8, n1, n2, 0x3) => self.xor(n1, n2),
            (0x8, n1, n2, 0x4) => self.add_reg(n1, n2),
            (0x8, n1, n2, 0x5) => self.sub(n1, n2),
            (0x8, n1, n2, 0x6) => self.shr(n1, n2),
            (0x8, n1, n2, 0x7) => self.subn(n1, n2),
            (0x8, n1, n2, 0xE) => self.shl(n1, n2),
            (0x9, n1, n2, 0x0) => self.sne_reg(n1, n2),
            (0xA, n1, n2, n3) => self.ld_i(n1, n2, n3),
            (0xB, n1, n2, n3) => {
                self.jmp_v0(n1, n2, n3);
                return;
            }
            (0xC, n1, n2, n3) => self.rnd(n1, n2, n3),
            (0xD, n1, n2, n3) => {
                if self.vblank {
                    self.drw(n1, n2, n3);
                    self.vblank = false;
                } else {
                    return;
                }
            }
            (0xE, n1, 0x9, 0xE) => self.skp(n1),
            (0xE, n1, 0xA, 0x1) => self.sknp(n1),
            (0xF, n1, 0x0, 0x7) => self.ld_delay(n1),
            (0xF, n1, 0x0, 0xA) => {
                if !self.wait_key(n1) {
                    return;
                }
            }
            (0xF, n1, 0x1, 0x5) => self.ld_todelay(n1),
            (0xF, n1, 0x1, 0xE) => self.add_i(n1),
            (0xF, n1, 0x3, 0x3) => self.bcd(n1),
            (0xF, n1, 0x5, 0x5) => self.ld_i_vx(n1),
            (0xF, n1, 0x6, 0x5) => self.ld_vx_i(n1),
            (a, b, c, d) => {
                println!("unknown -> {:x} : {:02x}{:02x} | {:x} {:x} {:x} {:x}", self.pc, ih, il, a, b, c, d);
                todo!();
            }
        }

        self.next();
    }

    fn next(&mut self) {
        self.pc += 2;
    }

    fn cls(&mut self) {
        for addr in &mut self.screen {
            *addr = OFF_COLOR;
        }
    }

    fn ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    fn jmp(&mut self, n1: u8, n2: u8, n3: u8) {
        let addr = ((n1 as u16) << 8) | (n2 << 4 | n3) as u16;
        self.pc = addr;
    }

    fn call(&mut self, n1: u8, n2: u8, n3: u8) {
        let addr = ((n1 as u16) << 8) | (n2 << 4 | n3) as u16;
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = addr;
    }

    fn se(&mut self, n1: u8, k1: u8, k2: u8) {
        let kk = k1 << 4 | k2;
        let vx = self.get_reg(n1);
        if vx == kk {
            self.next();
        }
    }

    fn sne(&mut self, n1: u8, k1: u8, k2: u8) {
        let kk = k1 << 4 | k2;
        let vx = self.get_reg(n1);
        if vx != kk {
            self.next();
        }
    }

    fn se_reg(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        if vx == vy {
            self.next();
        }
    }

    fn ld(&mut self, n1: u8, k1: u8, k2: u8) {
        let kk = k1 << 4 | k2;
        self.set_reg(n1, kk);
    }

    fn add(&mut self, n1: u8, k1: u8, k2: u8) {
        let kk = k1 << 4 | k2;
        let vx = self.get_reg(n1);
        let res = vx.overflowing_add(kk);
        self.set_reg(n1, res.0);
        if res.1 {
            self.set_reg(0xF, 1);
        } else {
            self.set_reg(0xF, 0);
        }
    }

    fn ld_reg(&mut self, n1: u8, n2: u8) {
        let vy = self.get_reg(n2);
        self.set_reg(n1, vy);
    }

    fn or(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        self.set_reg(n1, vx | vy);
        self.set_reg(0xF, 0);
    }

    fn and(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        self.set_reg(n1, vx & vy);
        self.set_reg(0xF, 0);
    }

    fn xor(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        self.set_reg(n1, vx ^ vy);
        self.set_reg(0xF, 0);
    }

    fn add_reg(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        let res = vx.overflowing_add(vy);
        self.set_reg(n1, res.0);
        if res.1 {
            self.set_reg(0xF, 1);
        } else {
            self.set_reg(0xF, 0);
        }
    }

    fn sub(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        let c = if vx >= vy { 1 } else { 0 };
        self.set_reg(n1, vx.wrapping_sub(vy));
        self.set_reg(0xF, c);
    }

    fn shr(&mut self, n1: u8, n2: u8) {
        let vy = self.get_reg(n2);

        let c = if vy & 0x01 == 1 { 1 } else { 0 };
        let res = vy.unbounded_shr(1);

        self.set_reg(n1, res);
        self.set_reg(0xF, c);
    }

    fn subn(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        let c = if vy >= vx { 1 } else { 0 };
        self.set_reg(n1, vy.wrapping_sub(vx));
        self.set_reg(0xF, c);
    }

    fn shl(&mut self, n1: u8, n2: u8) {
        let vy = self.get_reg(n2);

        let c = if vy & 0x80 == 0x80 { 1 } else { 0 };

        let res = vy.unbounded_shl(1);
        self.set_reg(n1, res);
        self.set_reg(0xF, c);
    }

    fn sne_reg(&mut self, n1: u8, n2: u8) {
        let vx = self.get_reg(n1);
        let vy = self.get_reg(n2);
        if vx != vy {
            self.next();
        }
    }

    fn ld_i(&mut self, n1: u8, n2: u8, n3: u8) {
        let addr = ((n1 as u16) << 8) | (n2 << 4 | n3) as u16;
        self.i =  addr;
    }

    fn jmp_v0(&mut self, n1: u8, n2: u8, n3: u8) {
        let addr = ((n1 as u16) << 8) | (n2 << 4 | n3) as u16;
        let v0 = self.get_reg(0);
        self.pc = addr + v0 as u16;
    }

    fn rnd(&mut self, n1: u8, k1: u8, k2: u8) {
        let kk = k1 << 4| k2;
        let r: u8 = self.rng.random();
        self.set_reg(n1, kk & r);
    }

    fn drw(&mut self, n1: u8, n2: u8, n3: u8) {
        let x = self.get_reg(n1);
        let y = self.get_reg(n2);

        let x = x.rem_euclid(WIDTH as u8);
        let y = y.rem_euclid(HEIGHT as u8);

        let mut c = 0;

        for a in 0..n3 {
            let y = y as usize + a as usize;
            if y >= HEIGHT {
                break;
            }

            let v = self.memory[(self.i + a as u16) as usize];

            for i in 0..8 {
                let x = x as usize + i as usize;
                if x >= WIDTH {
                    break;
                }

                let sa = y * WIDTH + x;
                let m = 0x80 >> i;
                let s = self.screen[sa];
                let va = v & m;

                if s == OFF_COLOR && va == m || s == ON_COLOR && va != m {
                    self.screen[sa] = ON_COLOR;
                } else {
                    self.screen[sa] = OFF_COLOR;
                }

                if s == ON_COLOR && self.screen[sa] == OFF_COLOR {
                    c = 1;
                }
            }
        }
        self.set_reg(0xF, c);
    }

    fn skp(&mut self, n1: u8) {
        let vx = self.get_reg(n1);
        if self.keys[vx as usize] {
            self.next();
        }
    }

    fn sknp(&mut self, n1: u8) {
        let vx = self.get_reg(n1);
        if !self.keys[vx as usize] {
            self.next();
        }
    }

    fn ld_delay(&mut self, n1: u8) {
        self.set_reg(n1, self.delay);
    }

    fn wait_key(&mut self, n1: u8) -> bool {
        for (i, k) in self.keys.iter().enumerate() {
            if *k {
                self.set_reg(n1, i as u8);
                return true;
            }
        }
        false
    }

    fn ld_todelay(&mut self, n1: u8) {
        let vx = self.get_reg(n1);
        self.delay = vx;
    }

    fn add_i(&mut self, n1: u8) {
        let vx = self.get_reg(n1);
        self.i += vx as u16;
    }

    fn bcd(&mut self, n1: u8) {
        let vx = self.get_reg(n1);
        self.memory[self.i as usize] = vx.div_euclid(100);
        let r = vx.rem_euclid(100);
        self.memory[self.i as usize + 1] = r.div_euclid(10);
        self.memory[self.i as usize + 2] = r.rem_euclid(10);
    }

    fn ld_i_vx(&mut self, n1: u8) {
        for i in 0..=n1 {
            let vx = self.get_reg(i);
            self.memory[self.i as usize] = vx;
            self.i += 1;
        }
    }

    fn ld_vx_i(&mut self, n1: u8) {
        for i in 0..=n1 {
            self.set_reg(i, self.memory[self.i as usize]);
            self.i += 1;
        }
    }
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
                .update_with_buffer(&chip8.screen, WIDTH, HEIGHT)
                .map_err(|_| Chip8Error::ScreenBuffer)?;
            chip8.set_vblank();
            delay_timer = Instant::now();
        }
    }

    Ok(())
}
