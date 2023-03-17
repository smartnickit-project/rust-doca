use crate::{DOCAError, DevContext};

use std::ptr::NonNull;
use std::sync::Arc;

/// Each DOCA Engine should implement their trait to
/// transfer the engine instance into a DOCA CTX instance
pub trait EngineToContext {
    unsafe fn to_ctx(&self) -> *mut ffi::doca_ctx;
}

/// DOCA context
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
    pub fn new(engine: &Arc<T>, added_devs: Vec<Arc<DevContext>>) -> Result<Arc<Self>, DOCAError> {
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
        self.stop().unwrap();

        for dev in &self.added_devs {
            self.remove_device(&dev)
                .expect("Failed to delete device from ctx");
        }

        // Show drop order only in `debug` mode
        #[cfg(debug_assertions)]
        println!("DOCA Context is dropped!");
    }
}

impl<T: EngineToContext> DOCAContext<T> {
    /// Finalizes all configurations, and starts the DOCA CTX.
    pub fn start(&mut self) -> Result<(), DOCAError> {
        let ret = unsafe { ffi::doca_ctx_start(self.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(())
    }

    /// Stops the context allowing reconfiguration.
    pub fn stop(&mut self) -> Result<(), DOCAError> {
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
    fn add_device(&mut self, dev: &Arc<DevContext>) -> Result<(), DOCAError> {
        let ret = unsafe { ffi::doca_ctx_dev_add(self.inner_ptr(), dev.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Remove a device from a DOCA CTX.
    /// Currently, it's used in `Drop` trait.
    // FIXME: `remove_device` needs a immutable reference
    // to pass the check in `drop` function
    fn remove_device(&self, dev: &Arc<DevContext>) -> Result<(), DOCAError> {
        let ret = unsafe { ffi::doca_ctx_dev_rm(self.inner_ptr(), dev.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(())
    }
}
