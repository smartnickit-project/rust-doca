use crate::buffer::{BufferInventory, DOCABuffer};
use crate::{DOCAMmap, RawPointer};

use ffi::doca_error;
use std::sync::Arc;
use std::ptr::NonNull;

/// A Simple struct to help manage the registered memory
///
/// Using DOCA memory is a two step process:
/// 1. populate it with `DOCAMmap::populate`
/// 2. allocate buffer with `DOCABufferInventory::alloc_buffer`.
///
pub struct DOCARegisteredMemory {
    mmap: Arc<DOCAMmap>,
    register_memory: RawPointer,
}

impl DOCARegisteredMemory {
    /// Create a new DOCARegisteredMemory
    pub fn new(
        mmap: &Arc<DOCAMmap>,
        register_memory: RawPointer,
    ) -> Result<Self, doca_error> {
        let mmap = mmap.clone();
        mmap.populate(
            unsafe { register_memory.get_inner().as_ptr() },
            register_memory.get_payload(),
        )?;

        Ok(Self {
            mmap,
            register_memory,
        })
    }

    /// Allocate a buffer from the registered memory
    pub fn to_buffer(self, inv: &Arc<BufferInventory>) -> Result<DOCABuffer, doca_error> {
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
