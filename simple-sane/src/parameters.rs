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

impl std::fmt::Display for Parameters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "resolution {width}x{height}, format '{format}' ({bpp} bpp)",
            width = self.pixels_per_line,
            height = self.lines,
            format = self.format,
            bpp = self.depth * (self.bytes_per_line / self.pixels_per_line),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FrameFormat {
    Gray,
    RGB,
    Red,
    Green,
    Blue,
}

impl std::fmt::Display for FrameFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameFormat::Gray => write!(f, "gray"),
            FrameFormat::RGB => write!(f, "RGB"),
            FrameFormat::Red => write!(f, "red"),
            FrameFormat::Green => write!(f, "green"),
            FrameFormat::Blue => write!(f, "blue"),
        }
    }
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
