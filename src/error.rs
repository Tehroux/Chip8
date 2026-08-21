#[derive(Debug)]
pub enum Chip8Error {
    Window,
    LoadingRom,
    ReadingRom,
    ScreenBuffer,
}
