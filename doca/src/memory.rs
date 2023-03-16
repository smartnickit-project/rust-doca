//! DOCA Memory subsystem
//!  
//! Memory is an important module in DOCA, which is for DOCA DMA.
//! Basically,the memory in DOCA is managed by a struct called `doca_mmap` that is a
//! memory pool that holding the memory regions the user register into it.
//! Also likeRDMA, every DMA request(src, dst) should be on these registered memory
//! regions.
//!
//!
use core::ffi::c_void;
use ffi::{doca_error, doca_mmap_populate};
use page_size;
use std::ptr::NonNull;
use std::sync::Arc;

use crate::device::DevContext;

const DOCA_MMAP_CHUNK_SIZE: u32 = 64; // 64 registered memory regions per mmap
/// A wrapper for `doca_mmap` struct
/// Since a mmap can be used by multiple device context,
/// we use a vector to record them.
///
pub struct DOCAMmap {
    // inner pointer of the doca memory pool
    inner: NonNull<ffi::doca_mmap>,
    // the device contexts that the doca memory pool registered
    ctx: Vec<Arc<DevContext>>,
}

impl Drop for DOCAMmap {
    fn drop(&mut self) {
        self.ctx.clear();
        self.stop().expect("failed to stop the doca_mmap");
        unsafe { ffi::doca_mmap_destroy(self.inner.as_ptr()) };
    }
}

impl DOCAMmap {
    /// Allocates a default mmap with default/unset attributes.
    /// This function should be called at server side.
    ///
    /// # Note
    ///   The default constructor will create a memory pool with maximum 64 chunks.
    ///
    /// Return values
    /// - DOCA_SUCCESS - in case of success. doca_error code - in case of failure:
    /// - DOCA_ERROR_INVALID_VALUE - if an invalid input had been received.
    /// - DOCA_ERROR_NO_MEMORY - failed to alloc doca_mmap.
    ///
    pub fn new() -> Result<Self, doca_error> {
        let mut pool: *mut ffi::doca_mmap = std::ptr::null_mut();

        // currently we don't use any user data
        let null_ptr: *mut ffi::doca_data = std::ptr::null_mut();

        let ret = unsafe { ffi::doca_mmap_create(null_ptr, &mut pool as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        let mut res = Self {
            inner: unsafe { NonNull::new_unchecked(pool) },
            ctx: Vec::new(),
        };
        res.set_max_chunks(DOCA_MMAP_CHUNK_SIZE)?;

        res.start()?;
        Ok(res)
    }

    /// TBD
    pub fn new_with_arg() {
        unimplemented!();
    }

    /// Return the inner pointer of the memory map object.
    #[inline]
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_mmap {
        self.inner.as_ptr()
    }

    /// Creates a memory map object representing the **remote** memory.
    /// It should be bound to a `DevContext`.
    ///
    /// Note that it is a remote device, so the usage should not be mixed with the local device.
    ///
    /// Return values
    /// - DOCA_SUCCESS - in case of success. doca_error code - in case of failure:
    /// - DOCA_ERROR_INVALID_VALUE - if an invalid input had been received or internal error. The following errors are internal and will occur if failed to produce new mmap from export descriptor:
    /// - DOCA_ERROR_NO_MEMORY - if internal memory allocation failed.
    /// - DOCA_ERROR_NOT_SUPPORTED - device missing create from export capability.
    /// - DOCA_ERROR_NOT_PERMITTED
    /// - DOCA_ERROR_DRIVER
    ///
    /// TODO: describe the input
    ///
    pub fn new_from_export(
        desc_buffer: *mut c_void,
        desc_len: usize,
        dev: &Arc<DevContext>,
    ) -> Result<Self, doca_error> {
        let mut pool: *mut ffi::doca_mmap = std::ptr::null_mut();
        // currently we don't use any user data
        let null_ptr: *mut ffi::doca_data = std::ptr::null_mut();

        let ret = unsafe {
            ffi::doca_mmap_create_from_export(
                null_ptr,
                desc_buffer,
                desc_len,
                dev.inner_ptr(),
                &mut pool as *mut _,
            )
        };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(Self {
            inner: unsafe { NonNull::new_unchecked(pool) },
            ctx: vec![dev.clone()],
        })
    }

    /// Export the **local mmap** information to a buffer.
    /// This buffer can be used by remote to create a new mmap,
    /// see the above `new_from_export`.
    ///
    /// Input:
    /// - dev_index: the index of the local device that the mmap is registered on.
    ///
    pub fn export(&self, dev_index: usize) -> Result<(*mut c_void, usize), doca_error> {
        let len: usize = 0;
        let len_ptr = &len as *const usize as *mut usize;

        let mut export_desc: *mut c_void = std::ptr::null_mut();
        let dev = self
            .ctx
            .get(dev_index)
            .ok_or(doca_error::DOCA_ERROR_INVALID_VALUE)?;

        let ret = unsafe {
            ffi::doca_mmap_export(
                self.inner_ptr(),
                dev.inner_ptr(),
                &mut export_desc as *mut _,
                len_ptr,
            )
        };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok((export_desc, len))
    }

    /// Register DOCA memory map on a given device.
    pub fn add_device(&mut self, dev: &Arc<DevContext>) -> Result<(), doca_error> {
        let ret = unsafe { ffi::doca_mmap_dev_add(self.inner_ptr(), dev.inner_ptr()) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        self.ctx.push(dev.clone());
        Ok(())
    }

    /// Deregister given device from DOCA memory map.
    /// You should call it before free the Memory Pool.
    pub fn rm_device(&self, _dev_idx: usize) -> Result<(), doca_error> {
        unimplemented!();
    }

    /// Add memory range to DOCA memory map.
    /// It is similar to `reg_mr` in RDMA.
    ///
    /// The memory can be used for DMA for all the contexts already in the mmap.
    ///
    pub fn populate(&self, addr: *mut c_void, len: usize) -> Result<(), doca_error> {
        let null_opaque: *mut c_void = std::ptr::null_mut::<c_void>();
        let ret = unsafe {
            doca_mmap_populate(
                self.inner_ptr(),
                addr,
                len,
                page_size::get(),
                None,
                null_opaque,
            )
        };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }
}

impl DOCAMmap {
    /// start the DOCA mmap
    /// TBD
    ///
    fn start(&self) -> Result<(), doca_error> {
        let ret = unsafe { ffi::doca_mmap_start(self.inner_ptr()) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// stop the DOCA mmap
    /// TBD
    ///
    fn stop(&self) -> Result<(), doca_error> {
        let ret = unsafe { ffi::doca_mmap_stop(self.inner_ptr()) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Set a new max number of chunks to populate in a DOCA Memory Map.
    /// Note: once a memory map object has been first started this functionality will not be available.
    ///
    fn set_max_chunks(&mut self, num: u32) -> Result<(), doca_error> {
        let ret = unsafe { ffi::doca_mmap_set_max_num_chunks(self.inner_ptr(), num) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }
}

mod tests {

    // a simple test to create a memory pool and 
    // register a memory on it
    #[test]
    fn test_memory_create() {

        use crate::*;

        // use the first device found 
        let device_ctx = devices().unwrap().get(0).unwrap().open().unwrap();
        let mut doca_mmap = DOCAMmap::new().unwrap();
        doca_mmap.add_device(&device_ctx).unwrap();

        let test_len = 1024;
        let mut dpu_buffer = vec![0u8; test_len].into_boxed_slice();
        doca_mmap.populate(dpu_buffer.as_mut_ptr() as _, test_len).unwrap();        
    }
}

