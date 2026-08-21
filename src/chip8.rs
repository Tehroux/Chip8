use std::{fs::File, io::Read, path::Path};

use rand::{RngExt, make_rng, rngs::SmallRng};

use crate::error::Chip8Error;

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

const MEMORY_SIZE: usize = 4096;
const STACK_SIZE: usize = 16;

const OFF_COLOR: u32 = 0xFF225522;
const ON_COLOR: u32 = 0xFF22AA22;

const REGISTER_COUNT: usize = 16;
const KEY_COUNT: usize = 16;

pub struct Chip8 {
    pub keys: Vec<bool>,
    vblank: bool,
    rng: SmallRng,
    screen: Vec<u32>,
    memory: Vec<u8>,
    stack: Vec<u16>,
    reg: Vec<u8>,
    i: u16,
    delay: u8,
    pc: u16,
    sp: u8,
}

impl Chip8 {
    pub fn new() -> Self {
        let mut memory = vec![
            0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80,
            0xF0, 0xF0, 0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0,
            0x10, 0xF0, 0xF0, 0x80, 0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90,
            0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0, 0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0,
            0x90, 0xE0, 0x90, 0xE0, 0xF0, 0x80, 0x80, 0x80, 0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0,
            0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
        ];

        memory.resize(MEMORY_SIZE, 0);

        Self {
            keys: vec![false; REGISTER_COUNT],
            screen: vec![OFF_COLOR; WIDTH * HEIGHT],
            memory,
            rng: make_rng(),
            stack: vec![0; STACK_SIZE],
            reg: vec![0; KEY_COUNT],
            pc: 0x200,
            i: 0,
            delay: 0,
            sp: 0,
            vblank: false,
        }
    }

    pub fn get_buffer(&self) -> &[u32] {
        self.screen.as_slice()
    }

    fn get_reg(&self, n: u8) -> u8 {
        self.reg[n as usize]
    }

    fn set_reg(&mut self, n: u8, value: u8) {
        self.reg[n as usize] = value;
    }

    pub fn load(&mut self, rom_path: &Path) -> Result<(), Chip8Error> {
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

    pub fn dec_delay(&mut self) {
        if self.delay > 0 {
            self.delay -= 1;
        }
    }

    pub fn set_vblank(&mut self) {
        self.vblank = true;
    }

    pub fn run(&mut self) {
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
            (0x3, n1, _, _) => self.se(n1, il),
            (0x4, n1, _, _) => self.sne(n1, il),
            (0x5, n1, n2, 0x0) => self.se_reg(n1, n2),
            (0x6, n1, _, _) => self.ld(n1, il),
            (0x7, n1, _, _) => self.add(n1, il),
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
            (0xC, n1, _, _) => self.rnd(n1, il),
            (0xD, n1, n2, n3) => {
                if !self.drw(n1, n2, n3) {
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
            (0xF, n1, 0x2, 0x9) => self.ld_d_i(n1),
            (0xF, n1, 0x3, 0x3) => self.bcd(n1),
            (0xF, n1, 0x5, 0x5) => self.ld_i_vx(n1),
            (0xF, n1, 0x6, 0x5) => self.ld_vx_i(n1),
            _ => {
                println!("unknown -> {:x} : {:02x}{:02x}", self.pc, ih, il);
                unimplemented!();
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

    fn se(&mut self, n1: u8, kk: u8) {
        let vx = self.get_reg(n1);
        if vx == kk {
            self.next();
        }
    }

    fn sne(&mut self, n1: u8, kk: u8) {
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

    fn ld(&mut self, n1: u8, kk: u8) {
        self.set_reg(n1, kk);
    }

    fn add(&mut self, n1: u8, kk: u8) {
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
        self.i = addr;
    }

    fn jmp_v0(&mut self, n1: u8, n2: u8, n3: u8) {
        let addr = ((n1 as u16) << 8) | (n2 << 4 | n3) as u16;
        let v0 = self.get_reg(0);
        self.pc = addr + v0 as u16;
    }

    fn rnd(&mut self, n1: u8, kk: u8) {
        let r: u8 = self.rng.random();
        self.set_reg(n1, kk & r);
    }

    fn drw(&mut self, n1: u8, n2: u8, n3: u8) -> bool {
        if !self.vblank {
            return false;
        }

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
        self.set_vblank();
        true
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

    fn ld_d_i(&mut self, n1: u8) {
        let vx = self.get_reg(n1);
        self.i = vx as u16 * 5;
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
