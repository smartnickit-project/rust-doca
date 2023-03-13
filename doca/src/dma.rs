//! DMA request related struct

use ffi::{doca_error::{self}, doca_dma_job_memcpy, DOCA_DMA_JOB_MEMCPY, DOCA_JOB_FLAGS_NONE};

use crate::{device::Device, memory::Buffer, context::Context};
/// DOCA DMA instance
pub struct DMAEngine {
    pub(crate) inner: *mut ffi::doca_dma
}

impl Drop for DMAEngine {
    fn drop(&mut self) {
        unsafe { ffi::doca_dma_destroy(self.inner) };
    }
}

impl DMAEngine {
    /// Create a DOCA DMA instance.
    pub fn new() -> Result<Self, doca_error::Type> {
        let mut dma: *mut ffi::doca_dma = std::ptr::null_mut();
        let ret = unsafe { ffi::doca_dma_create(&mut dma as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(
            Self { inner: dma }
        )
    }

    /// Get the maximum supported buffer size for DMA job.
    pub fn get_max_buf_size(dev: &Device) -> Result<u64, doca_error::Type> {
        let mut num: u64 = 0;
        let ret = unsafe { ffi::doca_dma_get_max_buf_size(dev.inner(), &mut num as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret)
        }

        Ok(num)
    }

    /// Create a new DOCA context based on the DMA instance.
    pub fn get_ctx(&self) -> Context {
        let ctx = unsafe { ffi::doca_dma_as_ctx(self.inner) };
        Context {
            inner: ctx
        }
    }

}


/// A DOCA DMA request
pub struct DMAJob {
    pub(crate) inner: doca_dma_job_memcpy
}

impl DMAJob {
    /// Create a DMA request
    pub fn new() -> Self {
        Self { 
            inner: doca_dma_job_memcpy::default()
        }
    }

    /// Set request's type
    pub fn set_type(&mut self) {
        self.inner.base.type_ = DOCA_DMA_JOB_MEMCPY as i32;
    }

    /// Set request's destination buffer
    pub fn set_dst(&mut self, buf: &Buffer) {
        self.inner.dst_buff = buf.inner()
    }

    /// Set request's source buffer
    pub fn set_src(&mut self, buf: &Buffer) {
        self.inner.src_buff = buf.inner()
    }

    /// Set request's based context
    pub fn set_ctx(&mut self, ctx: &Context) {
        self.inner.base.ctx = ctx.inner; 
    }

    /// Set request's flags
    pub fn set_flags(&mut self) {
        self.inner.base.flags = DOCA_JOB_FLAGS_NONE as i32;
    }

}