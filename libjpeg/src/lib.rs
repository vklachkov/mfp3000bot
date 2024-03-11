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

static mut JPEG: Vec<u8> = Vec::new();
const JPEG_BLOCK_SIZE: usize = 16384;

pub unsafe fn compress_to_jpeg(image: &RawImage, quality: u8) -> Vec<u8> {
    let mut cinfo: jpeg_compress_struct = std::mem::zeroed();
    let mut jerr: jpeg_error_mgr = std::mem::zeroed();

    cinfo.err = jpeg_std_error(&mut jerr);

    jpeg_CreateCompress(
        &mut cinfo,
        JPEG_LIB_VERSION as _,
        std::mem::size_of::<jpeg_compress_struct>(),
    );

    setup_destination(&mut cinfo);

    setup_image_parameters(&mut cinfo, image);
    jpeg_set_defaults(&mut cinfo);
    jpeg_set_quality(&mut cinfo, quality as _, 1);

    compress_pixels(&mut cinfo, &image.pixels);

    jpeg_destroy_compress(&mut cinfo);

    std::mem::take(&mut JPEG)
}

unsafe fn setup_destination(cinfo: &mut jpeg_compress_struct) {
    let alloc_small = (*cinfo.mem).alloc_small.unwrap();
    cinfo.dest = alloc_small(
        cinfo as *mut jpeg_compress_struct as *mut jpeg_common_struct,
        JPOOL_PERMANENT as _,
        std::mem::size_of::<jpeg_destination_mgr>(),
    ) as _;

    (*cinfo.dest).init_destination = Some(init_destination);
    (*cinfo.dest).empty_output_buffer = Some(empty_output_buffer);
    (*cinfo.dest).term_destination = Some(term_destination);
}

unsafe extern "C" fn init_destination(cinfo: *mut jpeg_compress_struct) {
    JPEG.resize(JPEG_BLOCK_SIZE, 0);

    (*(*cinfo).dest).next_output_byte = JPEG.as_mut_ptr();
    (*(*cinfo).dest).free_in_buffer = JPEG_BLOCK_SIZE;
}

unsafe extern "C" fn empty_output_buffer(cinfo: *mut jpeg_compress_struct) -> i32 {
    let oldsize = JPEG.len();

    JPEG.resize(oldsize + JPEG_BLOCK_SIZE, 0);

    (*(*cinfo).dest).next_output_byte = JPEG[oldsize..].as_mut_ptr();
    (*(*cinfo).dest).free_in_buffer = JPEG_BLOCK_SIZE;

    true as _
}

unsafe extern "C" fn term_destination(cinfo: *mut jpeg_compress_struct) {
    JPEG.resize(JPEG.len() - (*(*cinfo).dest).free_in_buffer, 0);
}

unsafe fn setup_image_parameters(cinfo: &mut jpeg_compress_struct, image: &RawImage) {
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

unsafe fn compress_pixels(cinfo: &mut jpeg_compress_struct, pixels: &[u8]) {
    jpeg_start_compress(cinfo, true as _);

    while cinfo.next_scanline < cinfo.image_height {
        let mut row = &pixels
            [(cinfo.next_scanline * cinfo.image_width * cinfo.input_components as u32) as usize]
            as *const u8 as *mut u8;

        jpeg_write_scanlines(cinfo, &mut row, 1);
    }

    jpeg_finish_compress(cinfo);
}
