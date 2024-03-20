use crate::{document::Document, job::Job, options::Options, utils::cstring_wrapper};
use libcups_sys::*;
use std::{
    ffi::CStr,
    io,
    ptr::{null, null_mut},
};

pub struct Printer(&'static mut cups_dest_t);

cstring_wrapper! { pub DeviceName }
cstring_wrapper! { pub JobTitle }

impl Printer {
    /// Get default printer for current user.
    pub fn get_default() -> Option<Self> {
        unsafe { cupsGetNamedDest(null_mut(), null(), null()).as_mut() }.map(Self)
    }

    /// Find printer by name.
    pub fn find_by_name(name: DeviceName) -> Option<Self> {
        unsafe { cupsGetNamedDest(null_mut(), name.as_ptr(), null()).as_mut() }.map(Self)
    }

    /// Get printer name.
    pub fn name(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.0.name) }
    }

    /// Send documents for printing.
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
            let document_name = document.name().clone();
            let last_document = documents.peek().is_none();

            match document.stream(self.name(), &job, last_document) {
                Ok(()) => {
                    log::debug!(
                        "Document '{document_name}' successfully uploaded to the job #{job_id} to the printer '{printer_name}'",
                        document_name = document_name.to_string_lossy(),
                        job_id = job.id,
                        printer_name = self.name().to_string_lossy()
                    );
                }
                Err(err) => {
                    unsafe { cupsCancelJob(self.name().as_ptr(), job.id) };
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    fn create_job(&self, title: &CStr, options: Options) -> io::Result<Job> {
        options.create_job(self.name(), title)
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        unsafe { cupsFreeDests(1, self.0) }
    }
}
