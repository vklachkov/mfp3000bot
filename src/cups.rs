use crate::cups_ffi as ffi;
use core::slice;
use cstr::cstr;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::fs;
use std::path::Path;
use std::ptr::{null, null_mut};
use std::time::Duration;

pub fn simple_print<P: AsRef<Path>>(title: &str, path: P) {
    let title = CString::new(title).expect("Title should not contain null bytes");

    use std::os::unix::ffi::OsStrExt;
    let file_path = CString::new(path.as_ref().as_os_str().as_bytes().to_vec())
        .expect("Path should not contain null bytes");

    let printer_name = cstr!("HP_Laser_MFP_135_WR");

    unsafe {
        let t = ffi::cupsPrintFile(
            printer_name.as_ptr(),
            file_path.as_ptr(),
            title.as_ptr(),
            0,
            null_mut(),
        );
    }
}

pub unsafe fn print_pdf<P: AsRef<Path>>(path: P) {
    let printer = get_default_printer();
    if printer.is_null() {
        return println!("No default printer!");
    };

    println!(
        "Successfully connect to the printer '{}'",
        CStr::from_ptr((*printer).name).to_string_lossy()
    );

    let (options, num_options) = setup_dest_options(printer);

    let Some(job_id) = create_job(printer, cstr!("title"), options, num_options) else {
        eprintln!("Failed to create job");
        print_cups_error();
        return;
    };

    println!("Successfully create job {job_id}");

    let path = path.as_ref();
    match fs::read(path) {
        Ok(content) => {
            print_document(
                printer,
                job_id,
                cstr!("cv.pdf"),
                &content,
                cstr!("application/pdf"),
                true,
            );
        }
        Err(err) => {
            eprintln!("Failed to read file '{path}': {err}", path = path.display());
        }
    }
}

unsafe fn get_default_printer() -> *mut ffi::cups_dest_t {
    return ffi::cupsGetNamedDest(null_mut(), null(), null());
}

unsafe fn connect_to_dest(dest: *mut ffi::cups_dest_t, timeout: Duration) -> *mut ffi::http_t {
    ffi::cupsConnectDest(
        dest,
        ffi::CUPS_DEST_FLAGS_DEVICE,
        c_int::try_from(timeout.as_millis()).unwrap_or(c_int::MAX),
        null_mut(),
        null_mut(),
        0,
        None,
        null_mut(),
    )
}

unsafe fn setup_dest_options(dest: *mut ffi::cups_dest_t) -> (*mut ffi::cups_option_t, c_int) {
    let mut options = null_mut();
    let mut count = 0;

    count = ffi::cupsAddOption(
        ffi::CUPS_COPIES.as_ptr() as *const c_char,
        cstr!("1").as_ptr(),
        count,
        &mut options,
    );

    count = ffi::cupsAddOption(
        ffi::CUPS_MEDIA.as_ptr() as *const c_char,
        ffi::CUPS_MEDIA_A4.as_ptr() as *const c_char,
        count,
        &mut options,
    );

    count = ffi::cupsAddOption(
        ffi::CUPS_SIDES.as_ptr() as *const c_char,
        ffi::CUPS_SIDES_ONE_SIDED.as_ptr() as *const c_char,
        count,
        &mut options,
    );

    (options, count)
}

unsafe fn create_job(
    dest: *mut ffi::cups_dest_t,
    title: &CStr,
    options: *mut ffi::cups_option_t,
    num_options: c_int,
) -> Option<c_int> {
    let job_id = ffi::cupsCreateJob(
        null_mut(),
        (*dest).name,
        title.as_ptr(),
        num_options,
        options,
    );

    if job_id > 0 {
        Some(job_id)
    } else {
        None
    }
}

unsafe fn print_document(
    dest: *mut ffi::cups_dest_t,
    job_id: c_int,
    file_name: &CStr,
    buffer: &[u8],
    format: &CStr,
    is_final_document: bool,
) {
    let start_dest_doc_status = ffi::cupsStartDocument(
        null_mut(),
        (*dest).name,
        job_id,
        file_name.as_ptr(),
        format.as_ptr(),
        if is_final_document { 1 } else { 0 },
    );

    if start_dest_doc_status != ffi::http_status_e_HTTP_STATUS_CONTINUE {
        eprintln!("Error from cupsStartDestDocument: {start_dest_doc_status}");
        print_cups_error();
        return;
    }

    let cups_write_request_data_status =
        ffi::cupsWriteRequestData(null_mut(), buffer.as_ptr() as *const _, buffer.len());

    if cups_write_request_data_status != ffi::http_status_e_HTTP_STATUS_CONTINUE {
        eprintln!("Error from cupsWriteRequestData: {cups_write_request_data_status}");
        print_cups_error();

        ffi::cupsFinishDocument(null_mut(), (*dest).name);
        ffi::cupsCancelJob((*dest).name, job_id);

        return;
    }

    if ffi::cupsFinishDocument(null_mut(), (*dest).name) == ffi::ipp_status_e_IPP_STATUS_OK {
        println!("Successfully send pdf to the printer!");
    } else {
        eprintln!("Failed to send pdf to the printer");
        print_cups_error();

        ffi::cupsCancelJob((*dest).name, job_id);
    }
}

fn print_cups_error() {
    let error = unsafe { CStr::from_ptr(ffi::cupsLastErrorString()) };
    eprintln!("CUPS error: {}", error.to_string_lossy());
}
