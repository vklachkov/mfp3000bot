use crate::ffi;

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

impl From<ffi::SANE_Frame> for FrameFormat {
    fn from(value: ffi::SANE_Frame) -> Self {
        match value {
            ffi::SANE_Frame_SANE_FRAME_GRAY => Self::Gray,
            ffi::SANE_Frame_SANE_FRAME_RGB => Self::RGB,
            ffi::SANE_Frame_SANE_FRAME_RED => Self::Red,
            ffi::SANE_Frame_SANE_FRAME_GREEN => Self::Green,
            ffi::SANE_Frame_SANE_FRAME_BLUE => Self::Blue,
            _ => panic!("invalid sane frame format {value}"),
        }
    }
}
