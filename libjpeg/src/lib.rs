use libjpeg_sys::{
    jpeg_CreateCompress, jpeg_common_struct, jpeg_compress_struct, jpeg_destination_mgr,
    jpeg_destroy_compress, jpeg_error_mgr, jpeg_finish_compress, jpeg_set_defaults,
    jpeg_set_quality, jpeg_start_compress, jpeg_std_error, jpeg_write_scanlines, JPEG_LIB_VERSION,
    JPOOL_PERMANENT, J_COLOR_SPACE_JCS_GRAYSCALE, J_COLOR_SPACE_JCS_RGB,
};

pub struct RawImage {
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub format: RawImageFormat,
}

#[derive(Clone, Copy)]
pub enum RawImageFormat {
    Rgb,
    Gray,
}

const JPEG_BLOCK_SIZE: usize = 16 * 1024;

pub fn compress_to_jpeg(image: &RawImage, quality: u8) -> Vec<u8> {
    let mut jpeg = Vec::new();

    let mut jerr = initialize_error_manager();
    let mut cinfo = initialize_encoder(&mut jerr, &mut jpeg);

    setup_destination(&mut cinfo);

    setup_image_parameters(&mut cinfo, image);

    unsafe { jpeg_set_defaults(&mut cinfo) };

    unsafe { jpeg_set_quality(&mut cinfo, quality as _, 1) };

    compress_pixels(&mut cinfo, &image.pixels);

    unsafe { jpeg_destroy_compress(&mut cinfo) };

    jpeg
}

fn initialize_error_manager() -> jpeg_error_mgr {
    // SAFETY: `jerr` будет корректно проинициализированно через `jpeg_std_error`.
    let mut jerr: jpeg_error_mgr = unsafe { std::mem::zeroed() };

    // SAFETY: Указатель на jpeg_error_mgr корректен.
    unsafe { jpeg_std_error(&mut jerr) };

    jerr
}

fn initialize_encoder(jerr: &mut jpeg_error_mgr, jpeg: &mut Vec<u8>) -> jpeg_compress_struct {
    // SAFETY: `cinfo` будет корректно проинициализировано через `jpeg_CreateCompress`.
    let mut cinfo: jpeg_compress_struct = unsafe { std::mem::zeroed() };

    cinfo.err = jerr;

    // SAFETY: Указатель на `jpeg_compress_struct` корректен.
    // TODO: Может ли `jpeg_CreateCompress` вернуть ошибку?
    unsafe {
        jpeg_CreateCompress(
            &mut cinfo,
            JPEG_LIB_VERSION as _,
            std::mem::size_of::<jpeg_compress_struct>(),
        )
    };

    cinfo.client_data = jpeg as *mut Vec<u8> as *mut _;

    cinfo
}

fn setup_destination(cinfo: &mut jpeg_compress_struct) {
    // SAFETY: поле `mem` должно быть корректно проинициализированно до вызова функции.
    let mem = unsafe { cinfo.mem.as_ref() }.expect("jpeg_compress_struct.mem should be non-null");

    let alloc_small = mem
        .alloc_small
        .expect("jpeg_memory_mgr.mem should be non-null");

    // SAFETY: JPEG аллокатор должен возвращать валидный указатель или null.
    let jpeg_destination_ptr = unsafe {
        alloc_small(
            cinfo as *mut jpeg_compress_struct as *mut jpeg_common_struct,
            JPOOL_PERMANENT as _,
            std::mem::size_of::<jpeg_destination_mgr>(),
        ) as *mut jpeg_destination_mgr
    };

    // SAFETY: JPEG аллокатор должен возвращать валидный указатель или null.
    let jpeg_destination = unsafe { jpeg_destination_ptr.as_mut() }
        .expect("failed to allocate memory for jpeg_destination_mgr in JPOOL_PERMANENT");

    jpeg_destination.init_destination = Some(init_destination);
    jpeg_destination.empty_output_buffer = Some(empty_output_buffer);
    jpeg_destination.term_destination = Some(term_destination);

    cinfo.dest = jpeg_destination;
}

unsafe extern "C" fn init_destination(cinfo: *mut jpeg_compress_struct) {
    let cinfo = cinfo.as_mut().unwrap();
    let dest = cinfo.dest.as_mut().unwrap();
    let buffer = cinfo.client_data.cast::<Vec<u8>>().as_mut().unwrap();

    buffer.resize(JPEG_BLOCK_SIZE, 0);

    dest.next_output_byte = buffer.as_mut_ptr();
    dest.free_in_buffer = JPEG_BLOCK_SIZE;
}

unsafe extern "C" fn empty_output_buffer(cinfo: *mut jpeg_compress_struct) -> i32 {
    let cinfo = cinfo.as_mut().unwrap();
    let dest = cinfo.dest.as_mut().unwrap();
    let buffer = cinfo.client_data.cast::<Vec<u8>>().as_mut().unwrap();

    let old_size = buffer.len();

    buffer.resize(old_size + JPEG_BLOCK_SIZE, 0);

    dest.next_output_byte = buffer[old_size..].as_mut_ptr();
    dest.free_in_buffer = JPEG_BLOCK_SIZE;

    true as _
}

unsafe extern "C" fn term_destination(cinfo: *mut jpeg_compress_struct) {
    let cinfo = cinfo.as_mut().unwrap();
    let dest = cinfo.dest.as_mut().unwrap();
    let buffer = cinfo.client_data.cast::<Vec<u8>>().as_mut().unwrap();

    buffer.resize(buffer.len() - dest.free_in_buffer, 0);
}

fn setup_image_parameters(cinfo: &mut jpeg_compress_struct, image: &RawImage) {
    cinfo.image_width = image.width as u32;
    cinfo.image_height = image.height as u32;

    match image.format {
        RawImageFormat::Rgb => {
            cinfo.input_components = 3;
            cinfo.in_color_space = J_COLOR_SPACE_JCS_RGB;
        }
        RawImageFormat::Gray => {
            cinfo.input_components = 1;
            cinfo.in_color_space = J_COLOR_SPACE_JCS_GRAYSCALE;
        }
    }
}

fn compress_pixels(cinfo: &mut jpeg_compress_struct, pixels: &[u8]) {
    // TODO: Могут ли start_compress, write_scanlines и finish_compress возвращать ошибки?

    unsafe { jpeg_start_compress(cinfo, true as _) };

    while cinfo.next_scanline < cinfo.image_height {
        let mut row = &pixels
            [(cinfo.next_scanline * cinfo.image_width * cinfo.input_components as u32) as usize]
            as *const u8 as *mut u8;

        unsafe { jpeg_write_scanlines(cinfo, &mut row, 1) };
    }

    unsafe { jpeg_finish_compress(cinfo) };
}
