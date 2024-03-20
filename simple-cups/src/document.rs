use crate::{
    job::Job,
    result::cups_error,
    utils::{c_enum, cstring_wrapper},
};
use libcups_sys::*;
use std::{ffi::CStr, io, ptr::null_mut};

/// Document for printing.
pub struct Document<'source> {
    name: DocumentName,
    ty: DocumentType,
    source: &'source mut dyn io::Read,
}

cstring_wrapper! {
    pub DocumentName
}

c_enum! {
    pub enum DocumentType {
        PlainText: CUPS_FORMAT_TEXT,
        Pdf: CUPS_FORMAT_PDF,
    }
}

impl<'source> Document<'source> {
    /// Create new document with given source.
    pub fn new(name: DocumentName, ty: DocumentType, source: &'source mut dyn io::Read) -> Self {
        Self { name, ty, source }
    }

    /// Document name.
    pub fn name(&self) -> &DocumentName {
        &self.name
    }

    /// Document type.
    pub fn ty(&self) -> DocumentType {
        self.ty
    }

    /// Read all bytes from source and stream them to CUPS' job.
    pub(crate) fn stream(
        mut self,
        device_name: &CStr,
        job: &Job,
        last_document: bool,
    ) -> io::Result<()> {
        self.start(device_name, job, last_document)?;
        self.write()?;
        self.finish(device_name)?;

        Ok(())
    }

    fn start(&self, device_name: &CStr, job: &Job, last_document: bool) -> io::Result<()> {
        let start_dest_doc_status = unsafe {
            cupsStartDocument(
                null_mut(),
                device_name.as_ptr(),
                job.id,
                self.name.as_ptr(),
                self.ty.value().as_ptr().cast(),
                if last_document { 1 } else { 0 },
            )
        };

        if start_dest_doc_status == http_status_e_HTTP_STATUS_CONTINUE {
            Ok(())
        } else {
            Err(io::Error::other(cups_error().unwrap()))
        }
    }

    fn write(&mut self) -> io::Result<()> {
        let capacity = 128 * 1024;
        let mut buffer = vec![0u8; capacity];

        loop {
            let read = self.source.read(&mut buffer)?;
            if read == 0 {
                break;
            }

            let status = unsafe { cupsWriteRequestData(null_mut(), buffer.as_ptr().cast(), read) };
            if status != http_status_e_HTTP_STATUS_CONTINUE {
                return Err(io::Error::other(cups_error().unwrap()));
            }
        }

        Ok(())
    }

    fn finish(&self, device_name: &CStr) -> io::Result<()> {
        let status = unsafe { cupsFinishDocument(null_mut(), device_name.as_ptr()) };

        if status == ipp_status_e_IPP_STATUS_OK {
            Ok(())
        } else {
            Err(io::Error::other(cups_error().unwrap()))
        }
    }
}
