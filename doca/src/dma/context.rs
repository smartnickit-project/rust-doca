use super::DMAEngine;
use crate::{DOCAError, DevContext};

use std::ptr::NonNull;
use std::sync::Arc;

/// DOCA DMA context
pub struct DOCAContext {
    inner: NonNull<ffi::doca_ctx>,
    #[allow(dead_code)]
    engine: Arc<DMAEngine>,
    #[allow(dead_code)]
    added_devs: Vec<Arc<DevContext>>,
}

impl DOCAContext {
    /// Create a new DOCA DMA context based on the DMA instance.
    pub fn new(
        dma: &Arc<DMAEngine>,
        added_devs: Vec<Arc<DevContext>>,
    ) -> Result<Arc<Self>, DOCAError> {
        assert!(!added_devs.is_empty());

        let ctx = unsafe { ffi::doca_dma_as_ctx(dma.inner_ptr()) };
        let mut res = Self {
            inner: unsafe { NonNull::new_unchecked(ctx) },
            engine: dma.clone(),
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

    /// Get the inner pointer of the DOCA DMA context.
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_ctx {
        self.inner.as_ptr()
    }
}

impl Drop for DOCAContext {
    fn drop(&mut self) {
        self.stop().expect("DOCAContext destroy should succeed");
    }
}

impl DOCAContext {
    /// Add a device to a DOCA CTX.
    #[inline]
    fn add_device(&mut self, dev: &Arc<DevContext>) -> Result<(), DOCAError> {
        let ret = unsafe { ffi::doca_ctx_dev_add(self.inner_ptr(), dev.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(())
    }

    /// Finalizes all configurations, and starts the DOCA CTX.
    fn start(&mut self) -> Result<(), DOCAError> {
        let ret = unsafe { ffi::doca_ctx_start(self.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(())
    }

    /// Stops the context allowing reconfiguration.
    fn stop(&mut self) -> Result<(), DOCAError> {
        let ret = unsafe { ffi::doca_ctx_stop(self.inner_ptr()) };
        if ret != DOCAError::DOCA_SUCCESS {
            return Err(ret);
        }
        Ok(())
    }
}

mod tests {

    #[test]
    fn test_dma_context() {
        use crate::dma::context::DOCAContext;
        use crate::dma::DMAEngine;

        let device = crate::device::devices().unwrap().get(0).unwrap().open().unwrap();

        let dma = DMAEngine::new().unwrap();
        let ctx = DOCAContext::new(&dma, vec![device]).unwrap();
        unsafe { assert_eq!(ctx.engine.inner_ptr(), dma.inner_ptr()) };
    }
}
