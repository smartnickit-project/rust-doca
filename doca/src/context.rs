//! DOCA Execution Related, including Context, WorkQueue and others.
//! 

use std::ptr::{NonNull};
use std::sync::Arc;

use ffi::{doca_error::{self, DOCA_ERROR_AGAIN}, doca_event, DOCA_WORKQ_RETRIEVE_FLAGS_NONE};
// use crate::{dma::{DMAEngine, DMAJob}, device::DevContext};

/// DOCA DMA CTX
pub struct DOCAContext {
    pub(crate) inner: NonNull<ffi::doca_ctx>
}

impl DOCAContext {
    /// Create a new DOCA DMA context based on the DMA instance.
    pub fn new(dma: &DMAEngine) -> Arc<Self> {
        let ctx = unsafe { ffi::doca_dma_as_ctx(dma.inner) };
        Self {
            inner: ctx
        }
    }

    /// Add a device to a DOCA CTX.
    pub fn add_device(&self, dev: &DevContext) -> doca_error::Type {
        unsafe { ffi::doca_ctx_dev_add(self.inner, dev.ctx()) }
    }

    /// Remove a device from a context.
    /// You should call the function before free `Context`
    pub fn rm_device(&self, dev: &DevContext) -> doca_error::Type {
        unsafe { ffi::doca_ctx_dev_rm(self.inner, dev.ctx()) }
    }

    /// Finalizes all configurations, and starts the DOCA CTX.
    pub fn start(&self) -> doca_error::Type {
        unsafe { ffi::doca_ctx_start(self.inner) }
    }

    /// Stops the context allowing reconfiguration.
    pub fn stop(&self) -> doca_error::Type {
        unsafe { ffi::doca_ctx_stop(self.inner) }
    }

    /// Get the ctx maximum number of contexts allowed within an application.
    pub fn get_max_num_ctx() -> Result<u32, doca_error::Type> {
        let mut num: u32 = 0;
        let ret = unsafe { ffi::doca_ctx_get_max_num_ctx(&mut num as *mut _)}; 

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret)
        }            
        
        Ok(num)
    }

    /// Add a Work Queue to the context.
    pub fn add_workq(&self, workq: &WorkQueue) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_ctx_workq_add(self.inner, workq.inner) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret)
        }

        Ok(())
    }

    /// Remove a DOCA Work Queue from a DOCA CTX.
    /// You should call the function before free `Context` & `WorkQueue`
    pub fn rm_workq(&self, workq: &WorkQueue) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_ctx_workq_rm(self.inner, workq.inner) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret)
        }

        Ok(())
    }

}
///Event structure defines activity completion of: 
/// 1. Completion event of submitted job. 
/// 2. CTX received event as a result of some external activity.
pub struct Event {
    inner: doca_event
}

impl Event {
    /// Get a DOCA Event Instance
    pub fn new() -> Self {
        Self {
            inner: doca_event::default()
        }
    }

    /// Get the return value of the event
    pub fn result(&self) -> u64 {
        unsafe { self.inner.result.u64 }
    }
}

/// a logical representation of DOCA thread of execution (non-thread-safe). 
/// WorkQ is used to submit jobs to the relevant context/library (hardware offload most of the time) 
/// and query the job's completion status. 
/// To start submitting jobs, however, the WorkQ must be configured to accept that type of job. 
/// Each WorkQ can be configured to accept any number of job types depending on how it initialized.
pub struct WorkQueue {
    inner: *mut ffi::doca_workq,
    depth: u32
}

impl Drop for WorkQueue {
    fn drop(&mut self) {
        unsafe { ffi::doca_workq_destroy(self.inner) };
    }
}

impl WorkQueue {
    /// Creates empty DOCA WorkQ object with default attributes.
    pub fn new(depth: u32) -> Result<Self,doca_error::Type> {
        let mut workq: *mut ffi::doca_workq = std::ptr::null_mut();
        let ret = unsafe { ffi::doca_workq_create(depth, &mut workq as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret)
        }

        Ok(
            Self {
                inner: workq,
                depth: depth
            }
        )
    }

    /// Add the job into the work queue
    pub fn submit(&self, job: &DMAJob) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_workq_submit(self.inner, &job.inner.base) };
        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret)
        }

        Ok(())
    }

    /// Check whether there's a job finished in the work queue
    pub fn poll_completion(&self, event: &mut Event) -> doca_error::Type {
        unsafe {
            let mut ret = ffi::doca_workq_progress_retrieve(self.inner, &mut event.inner as *mut _, DOCA_WORKQ_RETRIEVE_FLAGS_NONE as i32);

            while ret == DOCA_ERROR_AGAIN
            {
                ret = ffi::doca_workq_progress_retrieve(self.inner, &mut event.inner as *mut _, DOCA_WORKQ_RETRIEVE_FLAGS_NONE as i32);
            }

            ret
        }
    }

    /// Get the max depth of the work queue
    pub fn depth(&self) -> u32 {
        self.depth
    }
}

mod tests {

    #[test]
    fn test_dev_max_ctx() {
        let ret = crate::context::Context::get_max_num_ctx().unwrap();
        assert_ne!(ret, 0);
    }
}