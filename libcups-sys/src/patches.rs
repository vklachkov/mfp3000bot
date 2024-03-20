// This macro hides the original variable and changes the type from &[u8] to &Cstr,
// because bindgen doesn't support CStr:
// https://github.com/rust-lang/rust-bindgen/issues/2710.
macro_rules! patch_cstrings {
    ($($c:ident),* $(,)?) => {
        $(
            pub static $c: &::std::ffi::CStr =
                unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(crate::bindings::$c) };
        )*
    };
}

patch_cstrings!(
    CUPS_FORMAT_AUTO,
    CUPS_FORMAT_COMMAND,
    CUPS_FORMAT_JPEG,
    CUPS_FORMAT_PDF,
    CUPS_FORMAT_POSTSCRIPT,
    CUPS_FORMAT_RAW,
    CUPS_FORMAT_TEXT,
    CUPS_COPIES,
    CUPS_COPIES_SUPPORTED,
    CUPS_FINISHINGS,
    CUPS_FINISHINGS_SUPPORTED,
    CUPS_FINISHINGS_BIND,
    CUPS_FINISHINGS_COVER,
    CUPS_FINISHINGS_FOLD,
    CUPS_FINISHINGS_NONE,
    CUPS_FINISHINGS_PUNCH,
    CUPS_FINISHINGS_STAPLE,
    CUPS_FINISHINGS_TRIM,
    CUPS_MEDIA,
    CUPS_MEDIA_READY,
    CUPS_MEDIA_SUPPORTED,
    CUPS_MEDIA_3X5,
    CUPS_MEDIA_4X6,
    CUPS_MEDIA_5X7,
    CUPS_MEDIA_8X10,
    CUPS_MEDIA_A3,
    CUPS_MEDIA_A4,
    CUPS_MEDIA_A5,
    CUPS_MEDIA_A6,
    CUPS_MEDIA_ENV10,
    CUPS_MEDIA_ENVDL,
    CUPS_MEDIA_LEGAL,
    CUPS_MEDIA_LETTER,
    CUPS_MEDIA_PHOTO_L,
    CUPS_MEDIA_SUPERBA3,
    CUPS_MEDIA_TABLOID,
    CUPS_MEDIA_SOURCE,
    CUPS_MEDIA_SOURCE_SUPPORTED,
    CUPS_MEDIA_SOURCE_AUTO,
    CUPS_MEDIA_SOURCE_MANUAL,
    CUPS_MEDIA_TYPE,
    CUPS_MEDIA_TYPE_SUPPORTED,
    CUPS_MEDIA_TYPE_AUTO,
    CUPS_MEDIA_TYPE_ENVELOPE,
    CUPS_MEDIA_TYPE_LABELS,
    CUPS_MEDIA_TYPE_LETTERHEAD,
    CUPS_MEDIA_TYPE_PHOTO,
    CUPS_MEDIA_TYPE_PHOTO_GLOSSY,
    CUPS_MEDIA_TYPE_PHOTO_MATTE,
    CUPS_MEDIA_TYPE_PLAIN,
    CUPS_MEDIA_TYPE_TRANSPARENCY,
    CUPS_NUMBER_UP,
    CUPS_NUMBER_UP_SUPPORTED,
    CUPS_ORIENTATION,
    CUPS_ORIENTATION_SUPPORTED,
    CUPS_ORIENTATION_PORTRAIT,
    CUPS_ORIENTATION_LANDSCAPE,
    CUPS_PRINT_COLOR_MODE,
    CUPS_PRINT_COLOR_MODE_SUPPORTED,
    CUPS_PRINT_COLOR_MODE_AUTO,
    CUPS_PRINT_COLOR_MODE_MONOCHROME,
    CUPS_PRINT_COLOR_MODE_COLOR,
    CUPS_PRINT_QUALITY,
    CUPS_PRINT_QUALITY_SUPPORTED,
    CUPS_PRINT_QUALITY_DRAFT,
    CUPS_PRINT_QUALITY_NORMAL,
    CUPS_PRINT_QUALITY_HIGH,
    CUPS_SIDES,
    CUPS_SIDES_SUPPORTED,
    CUPS_SIDES_ONE_SIDED,
    CUPS_SIDES_TWO_SIDED_PORTRAIT,
    CUPS_SIDES_TWO_SIDED_LANDSCAPE,
);
