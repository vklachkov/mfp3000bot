use libsane_sys::*;

#[derive(Debug, Clone, Copy)]
pub struct Parameters {
    pub format: FrameFormat,
    pub last_frame: bool,
    pub bytes_per_line: usize,
    pub pixels_per_line: usize,
    pub lines: usize,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameFormat {
    Gray,
    RGB,
    Red,
    Green,
    Blue,
}

impl From<SANE_Frame> for FrameFormat {
    fn from(value: SANE_Frame) -> Self {
        match value {
            SANE_Frame_SANE_FRAME_GRAY => Self::Gray,
            SANE_Frame_SANE_FRAME_RGB => Self::RGB,
            SANE_Frame_SANE_FRAME_RED => Self::Red,
            SANE_Frame_SANE_FRAME_GREEN => Self::Green,
            SANE_Frame_SANE_FRAME_BLUE => Self::Blue,
            _ => panic!("invalid sane frame format {value}"),
        }
    }
}
