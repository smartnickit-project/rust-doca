use crate::{DOCABuffer, DOCAError};
use ffi::doca_event;
use std::ptr::NonNull;
use std::sync::Arc;

use super::DOCAContext;

///Event structure defines activity completion of:
/// 1. Completion event of submitted job.
/// 2. CTX received event as a result of some external activity.
#[derive(Default)]
#[repr(C)]
pub struct DOCAEvent {
    inner: doca_event,
}

impl DOCAEvent {
    /// Get a DOCA Event Instance
    pub fn new() -> Self {
        Self {
            inner: doca_event::default(),
        }
    }

    /// Get the return value of the event
    pub fn result(&self) -> DOCAError {
        unsafe {
            // FIXME: what if DOCAError is not u32?
            let res: DOCAError = std::mem::transmute(self.inner.result.u64 as u32);
            res
        }
    }
}

/// a logical representation of DOCA thread of execution (non-thread-safe).
/// WorkQ is used to submit jobs to the relevant context/library (hardware offload most of the time)
/// and query the job's completion status.
/// To start submitting jobs, however, the WorkQ must be configured to accept that type of job.
/// Each WorkQ can be configured to accept any number of job types depending on how it initialized.
pub struct DOCAWorkQueue {
    inner: NonNull<ffi::doca_workq>,
    depth: u32,
    #[allow(dead_code)]
    ctx: Arc<DOCAContext>,
}

impl Drop for DOCAWorkQueue {
    fn drop(&mut self) {
        // remove the worker queue from the context
        let ret = unsafe { ffi::doca_ctx_workq_rm(self.ctx.inner_ptr(), self.inner_ptr()) };
        assert_eq!(
            ret,
            DOCAError::DOCA_SUCCESS,
            "failed to remove workq from context"
        );
        unsafe { ffi::doca_workq_destroy(self.inner_ptr()) };
    }
}

impl DOCAWorkQueue {
    
    /// Creates empty DOCA WorkQ object with default attributes.
    pub fn new(depth: u32, ctx: &Arc<DOCAContext>) -> Result<Self, DOCAError> {
        let mut workq: *mut ffi::doca_workq = std::ptr::null_mut();
        let ret = unsafe { ffi::doca_workq_create(depth, &mut workq as *mut _) };

        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }

        let res = Self {
            inner: unsafe { NonNull::new_unchecked(workq) },
            depth: depth,
            ctx: ctx.clone(),
        };

        // add the myself to the context
        let ret = unsafe { ffi::doca_ctx_workq_add(ctx.inner_ptr(), res.inner_ptr()) };

        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(res)
    }

    /// Create a DMA job
    pub fn create_dma_job(&self, src_buf: DOCABuffer, dst_buf: DOCABuffer) -> super::DOCADMAJob {
        let mut res = super::DOCADMAJob {
            inner: Default::default(),
            ctx: self.ctx.clone(),
            src_buff: None,
            dst_buff: None,
        };
        res.set_ctx()
            .set_flags()
            .set_type()
            .set_src(src_buf)
            .set_dst(dst_buf);
        res
    }

    /// Add the job into the work queue
    pub fn submit(&mut self, job: &super::DOCADMAJob) -> Result<(), DOCAError> {
        let ret = unsafe { ffi::doca_workq_submit(self.inner_ptr(), &job.inner.base as *const _) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Check whether there's a job finished in the work queue
    #[inline]
    pub fn poll_completion(&mut self) -> Result<DOCAEvent, DOCAError> {
        let mut event = DOCAEvent::new();
        let ret = unsafe {
            ffi::doca_workq_progress_retrieve(
                self.inner_ptr(),
                &mut event.inner as *mut _,
                ffi::DOCA_WORKQ_RETRIEVE_FLAGS_NONE as i32,
            )
        };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(event)
    }

    /// Get the inner pointer of the DOCA WorkQ.
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_workq {
        self.inner.as_ptr()
    }

    /// Get the max depth of the work queue
    pub fn depth(&self) -> u32 {
        self.depth
    }
}

mod tests {
    #[test]
    fn test_worker_queue_create() {
        use crate::dma::DMAEngine;
        use crate::DOCAWorkQueue;

        let device = crate::device::devices()
            .unwrap()
            .get(0)
            .unwrap()
            .open()
            .unwrap();

        let ctx = DMAEngine::new()
            .unwrap()
            .create_context(vec![device])
            .unwrap();
        let workq = DOCAWorkQueue::new(1, &ctx).unwrap();

        assert_eq!(workq.depth(), 1);
    }
}
