use super::{ffi, options::Options};
use core::ffi::CStr;
use std::{
    io,
    ptr::{null, null_mut},
};

macro_rules! cstring_wrapper {
    ($($name:ident),* $(,)?) => {
        $(
            pub struct $name(::std::ffi::CString);

            impl $name {
                pub fn new(name: &str) -> Option<Self> {
                    let name = ::std::ffi::CString::new(name).ok()?;
                    Some(Self(name))
                }
            }

            impl ::std::ops::Deref for $name {
                type Target = ::std::ffi::CString;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )*
    };
}

cstring_wrapper!(DeviceName, JobTitle);

pub struct Printer(&'static mut ffi::cups_dest_t);

pub struct Document<'a> {
    pub file_name: DocumentName,
    pub ty: DocumentType,
    pub reader: &'a mut dyn io::Read,
}

cstring_wrapper!(DocumentName);

pub enum DocumentType {
    PlainText,
    PDF,
}

#[derive(Clone, Copy)]
pub struct JobId(i32);

impl Printer {
    pub fn get_default() -> Option<Self> {
        unsafe { ffi::cupsGetNamedDest(null_mut(), null(), null()).as_mut() }.map(Self)
    }

    pub fn find_by_name(name: DeviceName) -> Option<Self> {
        unsafe { ffi::cupsGetNamedDest(null_mut(), name.as_ptr(), null()).as_mut() }.map(Self)
    }

    pub fn name(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.0.name) }
    }

    pub fn print_documents(
        &self,
        title: JobTitle,
        options: Options,
        documents: Vec<Document>,
    ) -> io::Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let job = self.create_job(&title, options)?;

        let mut documents = documents.into_iter().peekable();
        while let Some(document) = documents.next() {
            let document_name = document.file_name.clone();
            let last_document = documents.peek().is_none();

            match document.send(self.name(), job, last_document) {
                Ok(()) => {
                    log::debug!(
                        "Document '{document_name}' successfully uploaded to the job #{job_id} to the printer '{printer_name}'",
                        document_name = document_name.to_string_lossy(),
                        job_id = job.0,
                        printer_name = self.name().to_string_lossy()
                    );
                }
                Err(err) => {
                    unsafe { ffi::cupsCancelJob(self.name().as_ptr(), job.0) };
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    fn create_job(&self, title: &CStr, options: Options) -> io::Result<JobId> {
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
            return Err(io::Error::other(cups_error().unwrap()));
        }

        Ok(JobId(job_id))
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        unsafe { ffi::cupsFreeDests(1, self.0) }
    }
}

impl Document<'_> {
    fn send(mut self, device_name: &CStr, job: JobId, last_document: bool) -> io::Result<()> {
        self.start(device_name, job, last_document)?;
        self.stream()?;
        self.finish(device_name)?;

        Ok(())
    }

    fn start(&self, device_name: &CStr, job: JobId, last_document: bool) -> io::Result<()> {
        let format: &[u8] = match self.ty {
            DocumentType::PlainText => ffi::CUPS_FORMAT_TEXT,
            DocumentType::PDF => ffi::CUPS_FORMAT_PDF,
        };

        let start_dest_doc_status = unsafe {
            ffi::cupsStartDocument(
                null_mut(),
                device_name.as_ptr(),
                job.0,
                self.file_name.as_ptr(),
                format.as_ptr().cast(),
                if last_document { 1 } else { 0 },
            )
        };

        if start_dest_doc_status == ffi::http_status_e_HTTP_STATUS_CONTINUE {
            Ok(())
        } else {
            Err(io::Error::other(cups_error().unwrap()))
        }
    }

    fn stream(&mut self) -> io::Result<()> {
        let capacity = 128 * 1024;
        let mut buffer = vec![0u8; capacity];

        loop {
            let read = self.reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }

            let status =
                unsafe { ffi::cupsWriteRequestData(null_mut(), buffer.as_ptr().cast(), read) };

            if status != ffi::http_status_e_HTTP_STATUS_CONTINUE {
                return Err(io::Error::other(cups_error().unwrap()));
            }
        }

        Ok(())
    }

    fn finish(&mut self, device_name: &CStr) -> io::Result<()> {
        let status = unsafe { ffi::cupsFinishDocument(null_mut(), device_name.as_ptr()) };

        if status == ffi::ipp_status_e_IPP_STATUS_OK {
            Ok(())
        } else {
            Err(io::Error::other(cups_error().unwrap()))
        }
    }
}

fn cups_error() -> Option<String> {
    let error = unsafe { ffi::cupsLastErrorString().as_ref() }?;

    let error = unsafe { CStr::from_ptr(error as *const i8) }
        .to_string_lossy()
        .into_owned();

    Some(error)
}
