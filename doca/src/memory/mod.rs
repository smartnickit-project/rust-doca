//! DOCA Memory subsystem
//!  
//! DOCA memory subsystem is designed to optimize performance while keeping a minimal memory footprint
//! (to facilitate scalability) as main design goals. DOCA memory is has two main components.
//!
//! - [`DOCABuffer`] represents the data buffer descriptor that the user wants to use.
//! There is also an entity called [`BufferInventory`] which serves as a pool of [`DOCABuffer`] with same characteristics.
//!
//! - [`DOCAMmap`] is the data buffers pool (chunks) which are pointed at by [`buffer`].
//! The application populates this memory pool with buffers/chunks and maps them to devices that must access the data.
//!
//! The way to use [`DOCAMmap`] is to register the memory the application might use into the object.
//!
//! ```
//! #![feature(get_mut_unchecked)]
//! use std::sync::Arc;
//! use doca::memory::DOCAMmap;
//! use doca::RawPointer;
//! use std::ptr::NonNull;
//! // Create a memory map object
//! let mut mmap = DOCAMmap::new().unwrap();
//!
//! // Allocate a buffer we want to use
//! let mut src_buffer = vec![0u8; 1024].into_boxed_slice();
//!
//! let mr = RawPointer {
//!     inner: NonNull::new(src_buffer.as_mut_ptr() as _).unwrap(),
//!     payload: 1024,
//! };
//!
//! // And register the buffer into the memory map object.
//! mmap.populate(mr).unwrap();
//! ```
pub mod buffer;
pub mod registered_memory;

use core::ffi::c_void;
use ffi::{doca_error, doca_mmap_populate};
use page_size;
use std::ptr::NonNull;
use std::sync::Arc;

use crate::device::DevContext;
use crate::{DOCAResult, RawPointer};

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
    // Control the drop behavior
    ok: bool,
}

// The `drop` function in DOCAMmap should be considered carefully.
// Since the operation `doca_mmap_dev_rm` is not permitted for:
// - un-started/stopped memory map object.
// - memory map object that have been exported or created from export.
// So in these situation, the `drop` function shouldn't call the `dev_rm` function:
// 1. The mmap is on the local side and exported;
// 2. The mmap is on the remote side and created by `new_from_export` on the local side;
impl Drop for DOCAMmap {
    fn drop(&mut self) {
        // Check whether the device should be removed
        if self.ok {
            for dev in &self.ctx {
                let ret = unsafe { ffi::doca_mmap_dev_rm(self.inner_ptr(), dev.inner_ptr()) };

                if ret != doca_error::DOCA_SUCCESS {
                    panic!(
                        "Failed to deregister the device from Memory Pool: {:?}",
                        ret
                    );
                }
            }
        }

        self.ctx.clear();
        unsafe { ffi::doca_mmap_destroy(self.inner.as_ptr()) };

        // Show drop order only in `debug` mode
        #[cfg(debug_assertions)]
        println!("DOCA mmap is dropped!");
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
    pub fn new() -> DOCAResult<Self> {
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
            ok: true,
        };
        res.set_max_chunks(DOCA_MMAP_CHUNK_SIZE)?;

        res.start()?;
        Ok(res)
    }

    // TBD
    // pub fn new_with_arg() {
    //     unimplemented!();
    // }

    /// Return the inner pointer of the memory map object.
    #[inline]
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_mmap {
        self.inner.as_ptr()
    }

    /// Creates a memory map object representing the **remote** memory.
    /// It should be bound to a `DevContext`.
    ///
    /// Note that it is a remote device, so the usage should not be mixed with the local device.
    /// The created object not backed by local memory.
    ///
    /// Limitation: Can only support mmap consisting of a single chunk.
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
    pub fn new_from_export(desc_buffer: RawPointer, dev: &Arc<DevContext>) -> DOCAResult<Self> {
        let mut pool: *mut ffi::doca_mmap = std::ptr::null_mut();
        // currently we don't use any user data
        let null_ptr: *mut ffi::doca_data = std::ptr::null_mut();

        let ret = unsafe {
            ffi::doca_mmap_create_from_export(
                null_ptr,
                desc_buffer.inner.as_ptr(),
                desc_buffer.payload,
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
            ok: false,
        })
    }

    /// Export the **local mmap** information to a buffer.
    /// This buffer can be used by remote to create a new mmap,
    /// see the above `new_from_export`.
    ///
    /// Input:
    /// - dev_index: the index of the local device that the mmap is registered on.
    ///
    pub fn export(&mut self, dev_index: usize) -> DOCAResult<RawPointer> {
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

        self.ok = false;

        Ok(RawPointer {
            inner: NonNull::new(export_desc).unwrap(),
            payload: len,
        })
    }

    /// Register DOCA memory map on a given device.
    pub fn add_device(&mut self, dev: &Arc<DevContext>) -> DOCAResult<usize> {
        let ret = unsafe { ffi::doca_mmap_dev_add(self.inner_ptr(), dev.inner_ptr()) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        self.ctx.push(dev.clone());
        Ok(self.ctx.len() - 1)
    }

    /// Deregister given device from DOCA memory map.
    /// Notice that, the given index from `add_device`
    /// will change after the user calls the function.
    pub fn rm_device(&self, _dev_idx: usize) -> DOCAResult<()> {
        let ret =
            unsafe { ffi::doca_mmap_dev_rm(self.inner_ptr(), self.ctx[_dev_idx].inner_ptr()) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Add memory range to DOCA memory map.
    /// It is similar to `reg_mr` in RDMA.
    ///
    /// The memory can be used for DMA for all the contexts already in the mmap.
    ///
    pub fn populate(&self, mr: RawPointer) -> DOCAResult<()> {
        let null_opaque: *mut c_void = std::ptr::null_mut::<c_void>();
        let ret = unsafe {
            doca_mmap_populate(
                self.inner_ptr(),
                mr.inner.as_ptr(),
                mr.payload,
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
    /// Allows execution of different operations on the mmap.
    ///
    fn start(&self) -> DOCAResult<()> {
        let ret = unsafe { ffi::doca_mmap_start(self.inner_ptr()) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Set a new max number of chunks to populate in a DOCA Memory Map.
    /// Note: once a memory map object has been first started this functionality will not be available.
    ///
    fn set_max_chunks(&mut self, num: u32) -> DOCAResult<()> {
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
        use std::ptr::NonNull;

        // use the first device found
        let device_ctx = devices().unwrap().get(0).unwrap().open().unwrap();
        let mut doca_mmap = DOCAMmap::new().unwrap();
        doca_mmap.add_device(&device_ctx).unwrap();

        let test_len = 1024;
        let mut dpu_buffer = vec![0u8; test_len].into_boxed_slice();
        let mr = RawPointer {
            inner: NonNull::new(dpu_buffer.as_mut_ptr() as _).unwrap(),
            payload: test_len,
        };

        doca_mmap.populate(mr).unwrap();
    }

    // Test show that the `rm_device` is forbidden on a exported mmap
    #[test]
    fn test_mmap_rm_device() {
        use crate::*;
        use std::ptr::NonNull;

        // use the first device found
        let device_ctx = devices().unwrap().get(0).unwrap().open().unwrap();
        let mut doca_mmap = DOCAMmap::new().unwrap();
        let dev_idx = doca_mmap.add_device(&device_ctx).unwrap();

        let test_len = 1024;
        let mut dpu_buffer = vec![0u8; test_len].into_boxed_slice();

        let mr = RawPointer {
            inner: NonNull::new(dpu_buffer.as_mut_ptr() as _).unwrap(),
            payload: test_len,
        };

        doca_mmap.populate(mr).unwrap();

        let _ = doca_mmap.export(dev_idx).unwrap();

        assert!(!doca_mmap.rm_device(dev_idx).is_ok());
    }
}
