//! Configuration type for [Pipeline](crate::pipeline::Pipeline).

use crate::{
    common::*,
    error::{ErrorChecker, Result},
    kind::{Format, StreamKind},
};

/// The pipeline configuration that will be consumed by [Pipeline::start()](crate::pipeline::Pipeline::start).
#[derive(Debug)]
pub struct Config {
    pub(crate) ptr: NonNull<sys::rs2_config>,
}

impl Config {
    /// Create an instance.
    pub fn new() -> Result<Self> {
        let ptr = unsafe {
            let mut checker = ErrorChecker::new();
            let ptr = sys::rs2_create_config(checker.inner_mut_ptr());
            checker.check()?;
            ptr
        };
        let config = Self {
            ptr: NonNull::new(ptr).unwrap(),
        };
        Ok(config)
    }

    /// Enable data stream with attributes.
    pub fn enable_stream(
        self,
        stream: StreamKind,
        index: usize,
        width: usize,
        height: usize,
        format: Format,
        framerate: usize,
    ) -> Result<Self> {
        unsafe {
            let mut checker = ErrorChecker::new();
            let ptr = sys::rs2_config_enable_stream(
                self.ptr.as_ptr(),
                stream as sys::rs2_stream,
                index as c_int,
                width as c_int,
                height as c_int,
                format as sys::rs2_format,
                framerate as c_int,
                checker.inner_mut_ptr(),
            );
            checker.check()?;
            ptr
        };
        Ok(self)
    }

    /// Enable device from a serial number.
    pub fn enable_device_from_serial(self, serial: &CStr) -> Result<Self> {
        unsafe {
            let mut checker = ErrorChecker::new();
            let ptr = sys::rs2_config_enable_device(
                self.ptr.as_ptr(),
                serial.as_ptr(),
                checker.inner_mut_ptr(),
            );
            checker.check()?;
            ptr
        };
        Ok(self)
    }

    /// Enable device from a file path.
    pub fn enable_device_from_file<P>(self, file: &CStr) -> Result<Self> {
        unsafe {
            let mut checker = ErrorChecker::new();
            let ptr = sys::rs2_config_enable_device_from_file(
                self.ptr.as_ptr(),
                file.as_ptr(),
                checker.inner_mut_ptr(),
            );
            checker.check()?;
            ptr
        };
        Ok(self)
    }

    pub fn into_raw(self) -> *mut sys::rs2_config {
        let ptr = self.ptr;
        mem::forget(self);
        ptr.as_ptr()
    }

    pub unsafe fn from_raw(ptr: *mut sys::rs2_config) -> Self {
        Self {
            ptr: NonNull::new(ptr).unwrap(),
        }
    }

    pub(crate) unsafe fn unsafe_clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        unsafe {
            sys::rs2_delete_config(self.ptr.as_ptr());
        }
    }
}

unsafe impl Send for Config {}
