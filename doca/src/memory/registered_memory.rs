//! Abstract of the memory in DOCA MMAP.
//!
//! The module contains a struct called [`DOCARegisteredMemory`] to
//! record these memory regions registered in the memory map object.
//! It holds the memory region metadata(start address and length) and
//! the memory map it belongs to.
//!
use crate::memory::buffer::{BufferInventory, DOCABuffer};
use crate::memory::DOCAMmap;
use crate::{DOCAResult, RawPointer};

use ffi::doca_error;
use std::ptr::NonNull;
use std::sync::Arc;

/// Using DOCA memory is a two step process:
/// 1. populate it with `DOCAMmap::populate`(Note that the remote address in a remote mmap has already been exported)
/// 2. allocate buffer with a `BufferInventory`.
///
pub struct DOCARegisteredMemory {
    mmap: Arc<DOCAMmap>,
    register_memory: RawPointer,
}

impl DOCARegisteredMemory {
    /// Create a new DOCARegisteredMemory
    pub fn new(mmap: &Arc<DOCAMmap>, register_memory: RawPointer) -> DOCAResult<Self> {
        let mmap = mmap.clone();
        mmap.populate(register_memory)?;

        Ok(Self {
            mmap,
            register_memory,
        })
    }

    /// Create a new DOCARegisteredMemory on the remote side
    pub fn new_from_remote(mmap: &Arc<DOCAMmap>, register_memory: RawPointer) -> DOCAResult<Self> {
        Ok(Self {
            mmap: mmap.clone(),
            register_memory: register_memory,
        })
    }

    /// Allocate a buffer from the registered memory
    pub fn to_buffer(self, inv: &Arc<BufferInventory>) -> DOCAResult<DOCABuffer> {
        let mut buffer: *mut ffi::doca_buf = std::ptr::null_mut();
        let ret = unsafe {
            ffi::doca_buf_inventory_buf_by_args(
                inv.inner_ptr(),
                self.mmap.inner_ptr(),
                self.register_memory.get_inner().as_ptr(), // head ptr
                self.register_memory.get_payload(),        // data payload
                self.register_memory.get_inner().as_ptr(), // data ptr
                0,                                         // data payload
                &mut buffer as *mut _,
            )
        };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(DOCABuffer {
            inner: unsafe { NonNull::new_unchecked(buffer) },
            head: self.register_memory,
            inv: inv.clone(),
            mmap: self.mmap,
        })
    }

    /// Get the `DOCAMmap` that was used to register the memory
    pub fn get_register_memory(&self) -> RawPointer {
        self.register_memory
    }
}
