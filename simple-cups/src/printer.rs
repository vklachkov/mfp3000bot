use super::{ffi, options::Options};
use core::ffi::CStr;
use std::{
    ffi::CString,
    io,
    ptr::{null, null_mut},
};
use thiserror::Error;

pub struct Printer {
    inner: *mut ffi::cups_dest_t,
}

pub struct Document<'a> {
    pub file_name: &'a str,
    pub ty: DocumentType,
    pub reader: &'a mut dyn io::Read,
}

pub enum DocumentType {
    PlainText,
    PDF,
}

#[derive(Clone, Copy)]
pub struct JobId(i32);

#[derive(Debug, Error)]
pub enum PrinterError {
    #[error("'{0}' contains zero bytes")]
    InvalidTitle(String),

    #[error("'{0}' contains zero bytes")]
    InvalidDocumentName(String),

    #[error("failed to read document: {0}")]
    ReadDocument(io::Error),

    #[error("failed to print document, cups error: {0}")]
    PrintDocument(String),
}

impl Printer {
    pub fn get_default() -> Option<Self> {
        let inner = unsafe { ffi::cupsGetNamedDest(null_mut(), null(), null()) };
        if inner.is_null() {
            return None;
        }

        Some(Self { inner })
    }

    pub fn find_by_name(name: &str) -> Option<Self> {
        let name = CString::new(name).unwrap();

        let inner = unsafe { ffi::cupsGetNamedDest(null_mut(), name.as_ptr(), null()) };
        if inner.is_null() {
            return None;
        }

        Some(Self { inner })
    }

    pub fn name(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.inner).name) }
    }

    pub fn print_documents(
        &self,
        title: &str,
        options: Options,
        documents: Vec<Document>,
    ) -> Result<(), PrinterError> {
        if documents.is_empty() {
            return Ok(());
        }

        let Ok(title) = CString::new(title.as_bytes()) else {
            return Err(PrinterError::InvalidTitle(title.to_owned()));
        };

        let job = self.create_job(&title, options)?;

        let last_index = documents.len() - 1;
        for (idx, document) in documents.into_iter().enumerate() {
            let Ok(docname) = CString::new(document.file_name.as_bytes()) else {
                return Err(PrinterError::InvalidDocumentName(
                    document.file_name.to_owned(),
                ));
            };

            let print_document = (|| {
                self.start_document(job, &docname, document.ty, idx == last_index)?;
                self.stream_document(document.reader)?;
                self.finish_document()
            })();

            if let Err(err) = print_document {
                unsafe { ffi::cupsCancelJob(self.name().as_ptr(), job.0) };
                return Err(err);
            }
        }

        Ok(())
    }

    fn create_job(&self, title: &CStr, options: Options) -> Result<JobId, PrinterError> {
        let (options, num_options) = options.into_raw();

        let job_id = unsafe {
            ffi::cupsCreateJob(
                null_mut(),
                self.name().as_ptr(),
                title.as_ptr(),
                num_options,
                options,
            )
        };

        if job_id == 0 {
            return Err(PrinterError::PrintDocument(Self::cups_error()));
        }

        Ok(JobId(job_id))
    }

    fn start_document(
        &self,
        job: JobId,
        docname: &CStr,
        doctype: DocumentType,
        last_document: bool,
    ) -> Result<(), PrinterError> {
        let format: &[u8] = match doctype {
            DocumentType::PlainText => ffi::CUPS_FORMAT_TEXT,
            DocumentType::PDF => ffi::CUPS_FORMAT_PDF,
        };

        let start_dest_doc_status = unsafe {
            ffi::cupsStartDocument(
                null_mut(),
                self.name().as_ptr(),
                job.0,
                docname.as_ptr(),
                format.as_ptr().cast(),
                if last_document { 1 } else { 0 },
            )
        };

        if start_dest_doc_status == ffi::http_status_e_HTTP_STATUS_CONTINUE {
            Ok(())
        } else {
            Err(PrinterError::PrintDocument(Self::cups_error()))
        }
    }

    fn stream_document(&self, reader: &mut dyn io::Read) -> Result<(), PrinterError> {
        let capacity = 128 * 1024;
        let mut buffer = vec![0u8; capacity];

        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(PrinterError::ReadDocument)?;

            if read == 0 {
                break;
            }

            let status =
                unsafe { ffi::cupsWriteRequestData(null_mut(), buffer.as_ptr().cast(), read) };

            if status != ffi::http_status_e_HTTP_STATUS_CONTINUE {
                return Err(PrinterError::PrintDocument(Self::cups_error()));
            }
        }

        Ok(())
    }

    fn finish_document(&self) -> Result<(), PrinterError> {
        let status = unsafe { ffi::cupsFinishDocument(null_mut(), self.name().as_ptr()) };

        if status == ffi::ipp_status_e_IPP_STATUS_OK {
            Ok(())
        } else {
            Err(PrinterError::PrintDocument(Self::cups_error()))
        }
    }

    fn cups_error() -> String {
        let error = unsafe { ffi::cupsLastErrorString() };
        if !error.is_null() {
            unsafe { CStr::from_ptr(error) }
                .to_string_lossy()
                .into_owned()
        } else {
            "no-error".to_owned()
        }
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        unsafe { ffi::cupsFreeDests(1, self.inner) }
    }
}
