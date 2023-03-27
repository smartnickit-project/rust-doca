//! DOCA Execution models.
//!
//! The DOCA Execution models mainly contains two components.
//! - [`DOCAContext`] is the base class of every data-path library in DOCA.
//! It is a specific library/SDK instance object providing abstract data processing functionality.
//! The library exposes events and/or jobs that manipulate data.
//!
//! Since each data-path library has its
//! own context, the trait [`EngineToContext`] is designed for these libraries to implement their
//! own function to transfer an data-path engine into a Context instance.  For example, to submit DMA jobs,
//! a DMA context can be acquired from [`DMAEngine`], whereas SHA context can be obtained using another implementation.
//!
//! - [`DOCAWorkQueue`]  is a per-thread object used to queue jobs to
//! offload to DOCA and eventually receive their completion status.
//!

use crate::{DOCAError, DOCAResult, DevContext};

use std::ptr::NonNull;
use std::sync::Arc;

/// Each DOCA Engine should implement their trait to
/// transfer the engine instance into a DOCA CTX instance
pub trait EngineToContext {
    /// Get a DOCA CTX from a DOCA Engine instance
    unsafe fn to_ctx(&self) -> *mut ffi::doca_ctx;
}

/// DOCA context
/// DOCAContext is a thread-safe object.
pub struct DOCAContext<T: EngineToContext> {
    inner: NonNull<ffi::doca_ctx>,

    // Ensure that the engine should be dropped after the context is dropped
    #[allow(dead_code)]
    pub(crate) engine: Arc<T>,
    #[allow(dead_code)]
    added_devs: Vec<Arc<DevContext>>,
}

impl<T: EngineToContext> DOCAContext<T> {
    /// Create a new DOCA context based on the Engine instance.
    pub fn new(engine: &Arc<T>, added_devs: Vec<Arc<DevContext>>) -> DOCAResult<Arc<Self>> {
        assert!(!added_devs.is_empty());

        let mut res = Self {
            inner: unsafe { NonNull::new_unchecked(engine.to_ctx()) },
            engine: engine.clone(),
            added_devs: Vec::new(),
        };

        // add device to it
        for dev in &added_devs {
            res.add_device(dev)?;
        }
        res.added_devs = added_devs;

        // start the context
        res.start()?;

        Ok(Arc::new(res))
    }
}

impl<T: EngineToContext> Drop for DOCAContext<T> {
    fn drop(&mut self) {
        let _ = self.stop().map_err(|e| {
            panic!("Failed to stop the Context: {:?}", e);
        });

        for dev in &self.added_devs {
            let ret = unsafe { ffi::doca_ctx_dev_rm(self.inner_ptr(), dev.inner_ptr()) };
            if ret != DOCAError::DOCA_SUCCESS {
                panic!("Failed to remove device from the context: {:?}", ret);
            }
        }

        // Show drop order only in `debug` mode
        #[cfg(debug_assertions)]
        println!("DOCA Context is dropped!");
    }
}

impl<T: EngineToContext> DOCAContext<T> {
    /// Finalizes all configurations, and starts the DOCA CTX.
    pub fn start(&mut self) -> DOCAResult<()> {
        let ret = unsafe { ffi::doca_ctx_start(self.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(())
    }

    /// Stops the context allowing reconfiguration.
    pub fn stop(&mut self) -> DOCAResult<()> {
        let ret = unsafe { ffi::doca_ctx_stop(self.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(())
    }

    /// Get the inner pointer of the DOCA context.
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_ctx {
        self.inner.as_ptr()
    }
}

impl<T: EngineToContext> DOCAContext<T> {
    /// Add a device to a DOCA CTX.
    #[inline]
    fn add_device(&mut self, dev: &Arc<DevContext>) -> DOCAResult<()> {
        let ret = unsafe { ffi::doca_ctx_dev_add(self.inner_ptr(), dev.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }
}

/// WorkQueue
pub mod work_queue;
