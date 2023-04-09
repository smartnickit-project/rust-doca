//! Wrapper over `doca_buf` and its related structs.
//!
//! The module mainly contains two components of DOCA
//! - [`DOCABuffer`]  is used for reference data.
//! It holds the information on a memory region that belongs to a DOCA memory map,
//! and its descriptor is allocated from DOCA Buffer Inventory.
//!
//! - [`BufferInventory`] manages a pool of doca_buf objects.
//! Each buffer obtained from an inventory is a descriptor that
//! points to a memory region from a doca_mmap memory range of the user's choice.
//!
//! The module also provides an abstraction of the data stored in a memory map [`RawPointer`].
//!
//! The usage of this module is to create a specific mmap buffer which points to a piece of
//! memory in the memory map:
//! ```
//! use std::sync::Arc;
//! use doca::memory::DOCAMmap;
//! use doca::memory::buffer::BufferInventory;
//! use doca::DOCARegisteredMemory;
//! use doca::RawPointer;
//!
//! // The memory region we want to register into the memory map
//! let mut mem_buffer = vec![0u8; 1024].into_boxed_slice();
//! // Create the memory map object
//! let mmap = Arc::new(DOCAMmap::new().unwrap());
//! // Create the buffer inventory for buffer allocation
//! let inv = BufferInventory::new(1024).unwrap();
//!
//! // Register the memory region into the memory map
//! let mut dma_buffer = DOCARegisteredMemory::new(&mmap, unsafe { RawPointer::from_box(&mem_buffer) })
//!     .unwrap()
//!     // And get a buffer pointing to it
//!     .to_buffer(&inv)
//!     .unwrap();
//!
//! ```
use core::ffi::c_void;
use ffi::doca_error;
use std::ptr::NonNull;
use std::sync::Arc;
use std::convert::From;

use crate::memory::DOCAMmap;
use crate::DOCAResult;

use serde_derive::{Deserialize, Serialize};

/// An abstraction of raw pointer pointing to a given buffer size:
/// inner -> |   ....  payload .... |
///
#[derive(Clone, Copy)]
pub struct RawPointer {
    /// The header of the data
    pub inner: NonNull<c_void>,
    /// The data length
    pub payload: usize,
}

/// a (de)serializable struct for passing RawPointer between nodes
#[derive(Serialize, Deserialize)]
pub struct RawPointerMsg {
    inner: u64,
    payload: usize,
}

impl RawPointer {
    /// get the raw inner pointer
    pub unsafe fn get_inner(&self) -> NonNull<c_void> {
        self.inner
    }

    /// get the payload size`
    pub fn get_payload(&self) -> usize {
        self.payload
    }

    /// get the raw pointer from a box
    /// it is unsafe because we extra create a raw pointer from the box
    pub unsafe fn from_box(boxed: &Box<[u8]>) -> Self {
        Self {
            inner: NonNull::new_unchecked(boxed.as_ptr() as _),
            payload: boxed.len(),
        }
    }

    /// get the raw pointer from a pointer
    /// Usually, it's used to present a remote memory region
    pub unsafe fn from_raw_ptr(ptr: *mut u8, len: usize) -> Self {
        Self {
            inner: NonNull::new_unchecked(ptr as _),
            payload: len,
        }
    }
}

impl From<RawPointerMsg> for RawPointer {
    fn from(msg: RawPointerMsg) -> Self {
        Self {
            inner: NonNull::new(msg.inner as *mut c_void).unwrap(),
            payload: msg.payload,
        }
    }
}

impl RawPointerMsg {
    /// convert a RawPointerMsg to Vec<u8> for socket sending
    #[inline]
    pub fn serialize(src: RawPointerMsg) -> Vec<u8> {
        serde_json::to_vec(&src).unwrap()
    }

    /// convert a u8 slice recved to RawPointerMsg
    #[inline]
    pub fn deserialize(src: &[u8]) -> RawPointerMsg {
        serde_json::from_slice(src).unwrap()
    }
}

impl From<RawPointer> for RawPointerMsg {
    fn from(ptr: RawPointer) -> Self {
        let inner_ptr = ptr.inner.as_ptr();
        Self {
            inner: inner_ptr as u64,
            payload: ptr.payload,
        }
    }
}

/// The DOCA Buffer is used for reference data.
/// It holds the information on a memory region that belongs to a DOCA memory map,
/// and its descriptor is allocated from DOCA Buffer Inventory.
///
pub struct DOCABuffer {
    pub(crate) inner: NonNull<ffi::doca_buf>,
    pub(crate) head: RawPointer,

    // FIXME: it would be safe to record references to the creators
    // However, it may add extra overhead to the structures.
    #[allow(dead_code)]
    pub(crate) inv: Arc<BufferInventory>,
    #[allow(dead_code)]
    pub(crate) mmap: Arc<DOCAMmap>,
}

impl Drop for DOCABuffer {
    fn drop(&mut self) {
        let ret = unsafe { ffi::doca_buf_refcount_rm(self.inner_ptr(), std::ptr::null_mut()) };
        if ret != doca_error::DOCA_SUCCESS {
            panic!("Failed to remove refcount of doca buffer");
        }

        // Show drop order only in `debug` mode
        #[cfg(debug_assertions)]
        println!("DOCA Buffer is dropped!");
    }
}

impl DOCABuffer {
    /// Get the buffer's data.
    /// It is unsafe because we don't track the lifetime of the returned pointer.
    ///
    pub unsafe fn get_data(&self) -> DOCAResult<*mut c_void> {
        let mut data: *mut c_void = std::ptr::null_mut();

        let ret = unsafe { ffi::doca_buf_get_data(self.inner_ptr(), &mut data as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(data)
    }

    /// Set data pointer and data length
    /// The data pointer and length should fix in the head region.
    /// Therefore, we adopt usize (in offset), instead of passing the raw pointers
    pub unsafe fn set_data(&mut self, off: usize, sz: usize) -> DOCAResult<()> {
        let ret = unsafe {
            ffi::doca_buf_set_data(
                self.inner_ptr(),
                (self.head.get_inner().as_ptr() as *mut u8).offset(off as _) as _,
                sz,
            )
        };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Return the pointer
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_buf {
        self.inner.as_ptr()
    }
}

/// The DOCA buffer inventory manages a pool of doca_buf objects.
/// Each buffer obtained from an inventory is a descriptor that points to a memory region from a doca_mmap memory range of the user's choice.
pub struct BufferInventory {
    inner: NonNull<ffi::doca_buf_inventory>,
}

impl Drop for BufferInventory {
    fn drop(&mut self) {
        unsafe { ffi::doca_buf_inventory_destroy(self.inner.as_ptr()) };

        // Show drop order only in `debug` mode
        #[cfg(debug_assertions)]
        println!("Buffer Inventory is dropped!");
    }
}

impl BufferInventory {
    /// Allocates buffer inventory with default/unset attributes.
    ///
    /// # Input:
    /// - `num` - number of elements in the inventory.
    ///
    /// FIXME: currently we omit setting other attributes of the inventory.
    ///
    pub fn new(num: usize) -> DOCAResult<Arc<Self>> {
        // currently we don't use `user_data` field
        let mut buf_inv: *mut ffi::doca_buf_inventory = std::ptr::null_mut();
        // DOCA_BUF_EXTENSION_NONE = 0;
        let ret = unsafe {
            ffi::doca_buf_inventory_create(std::ptr::null(), num, 0, &mut buf_inv as *mut _)
        };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        let mut res = Self {
            inner: unsafe { NonNull::new_unchecked(buf_inv) },
        };
        res.start()?;

        Ok(Arc::new(res))
    }

    /// Return the pointer
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_buf_inventory {
        self.inner.as_ptr()
    }

    /// Start element retrieval from inventory.
    fn start(&mut self) -> DOCAResult<()> {
        let ret = unsafe { ffi::doca_buf_inventory_start(self.inner_ptr()) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }
}

mod tests {
    #[allow(unused_imports)]
    use crate::{memory::registered_memory, DOCARegisteredMemory};

    #[test]
    fn test_basic_buffer_inv() {
        use super::*;
        use crate::memory::DOCAMmap;

        let doca_mmap = Arc::new(DOCAMmap::new().unwrap());
        let inv = BufferInventory::new(1024).unwrap();

        let test_len = 64;
        let mut dpu_buffer = vec![0u8; test_len].into_boxed_slice();

        let raw_pointer = RawPointer {
            inner: NonNull::new(dpu_buffer.as_mut_ptr() as _).unwrap(),
            payload: test_len,
        };

        let registered_memory = DOCARegisteredMemory::new(&doca_mmap, raw_pointer).unwrap();
        let buf = registered_memory.to_buffer(&inv).unwrap();

        let data = unsafe { buf.get_data().unwrap() };
        assert_eq!(data, dpu_buffer.as_ptr() as *mut c_void);
    }
}
