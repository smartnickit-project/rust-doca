//! Wrapper for DOCA DMA related.
//! The core structs include [`DOCADMAJob`], [`DMAEngine`], [`DOCAEvent`], [`DOCAWorkQueue`] and [`DOCAContext`].
//!
//! It basically contains two modules:
//! - [`context`]: DOCA DMA context related.
//! - [`work_queue`]: DOCA DMA work queue related.
//!

use std::ptr::NonNull;
use std::sync::Arc;

use crate::{DOCABuffer, DOCAError, DevContext};

pub mod context;
pub mod work_queue;

pub use context::DOCAContext;
pub use work_queue::{DOCAEvent, DOCAWorkQueue};

/// A DOCA DMA request
pub struct DOCADMAJob {
    pub(crate) inner: ffi::doca_dma_job_memcpy,

    // FIXME: do we really need to record the context here?
    #[allow(dead_code)]
    ctx: Arc<DOCAContext>,

    src_buff: Option<DOCABuffer>,
    dst_buff: Option<DOCABuffer>,
}

impl DOCADMAJob {
    /// Set request's destination buffer
    pub fn set_dst(&mut self, buf: DOCABuffer) -> &mut Self {
        unsafe { self.inner.dst_buff = buf.inner_ptr() };
        self.dst_buff = Some(buf);
        self
    }

    /// Set request's source buffer
    pub fn set_src(&mut self, buf: DOCABuffer) -> &mut Self {
        unsafe { self.inner.src_buff = buf.inner_ptr() };
        self.src_buff = Some(buf);
        self
    }

    /// Set request's based context
    fn set_ctx(&mut self) -> &mut Self {
        unsafe { self.inner.base.ctx = self.ctx.inner_ptr() };
        self
    }

    /// Set request's flags
    fn set_flags(&mut self) -> &mut Self {
        self.inner.base.flags = ffi::DOCA_JOB_FLAGS_NONE as i32;
        self
    }

    /// Set request's type
    fn set_type(&mut self) -> &mut Self {
        self.inner.base.type_ = ffi::DOCA_DMA_JOB_MEMCPY as i32;
        self
    }
}

/// DOCA DMA engine instance
pub struct DMAEngine {
    inner: NonNull<ffi::doca_dma>,
}

impl DMAEngine {
    /// Create a DOCA DMA instance.
    pub fn new() -> Result<Arc<Self>, DOCAError> {
        let mut dma: *mut ffi::doca_dma = std::ptr::null_mut();
        let ret = unsafe { ffi::doca_dma_create(&mut dma as *mut _) };

        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(Arc::new(Self {
            inner: unsafe { NonNull::new_unchecked(dma) },
        }))
    }

    /// Create a DMA context based on the DMA instance.
    pub fn create_context(
        self: &Arc<Self>,
        added_devs: Vec<Arc<DevContext>>,
    ) -> Result<Arc<DOCAContext>, DOCAError> {
        DOCAContext::new(self, added_devs)
    }

    /// Get the inner pointer of the DOCA DMA instance.
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_dma {
        self.inner.as_ptr()
    }
}


mod tests { 
    #[test]
    fn test_create_dma_job() { 
        use crate::dma::DMAEngine;
        use crate::*;        
        use super::*;
        use std::ptr::NonNull;

        let device = crate::device::devices().unwrap().get(0).unwrap().open().unwrap();

        let ctx = DMAEngine::new().unwrap().create_context(vec![device]).unwrap();
        let workq = DOCAWorkQueue::new(1, &ctx).unwrap();        

        // create buffers 
        let doca_mmap = Arc::new(DOCAMmap::new().unwrap());
        let inv = BufferInventory::new(1024).unwrap();

        let test_len = 64;
        let mut dpu_buffer = vec![0u8; test_len].into_boxed_slice();
        let mut dpu_buffer_1 = vec![0u8; test_len].into_boxed_slice();

        let raw_pointer = RawPointer {
            inner: NonNull::new(dpu_buffer.as_mut_ptr() as _).unwrap(),
            payload: test_len,
        };

        let raw_pointer_1 = RawPointer {
            inner: NonNull::new(dpu_buffer_1.as_mut_ptr() as _).unwrap(),
            payload: test_len,
        };        

        let registered_memory = DOCARegisteredMemory::new(&doca_mmap, raw_pointer).unwrap();
        let src_buf = registered_memory.to_buffer(&inv).unwrap();

        let registered_memory = DOCARegisteredMemory::new(&doca_mmap, raw_pointer_1).unwrap();
        let dst_buf = registered_memory.to_buffer(&inv).unwrap();

        let _ = workq.create_dma_job(src_buf, dst_buf);

    }
}