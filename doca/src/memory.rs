//! DOCA Memory subsystem
//!  
use ffi::{doca_error, doca_mmap_populate};
use core::ffi::c_void;
use page_size;

use crate::device::DevContext;

/// The DOCA memory map provides a centralized repository and 
/// orchestration of several memory ranges registration for each 
/// device attached to the memory map.
pub struct MemoryPool {
    pool: *mut ffi::doca_mmap
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        unsafe { ffi::doca_mmap_destroy(self.pool) };
    }
}

impl MemoryPool {
    /// Allocates zero size memory map object with default/unset attributes.
    /// This function should be called at server side
    /// 
    /// Return values
    /// DOCA_SUCCESS - in case of success. doca_error code - in case of failure:
    /// DOCA_ERROR_INVALID_VALUE - if an invalid input had been received.
    /// DOCA_ERROR_NO_MEMORY - failed to alloc doca_mmap.
    pub fn new() -> Result<Self, doca_error::Type> {
        let mut pool: *mut ffi::doca_mmap = std::ptr::null_mut();

        // currently we don't use any user data
        let null_ptr: *mut ffi::doca_data = std::ptr::null_mut();

        let ret = unsafe { ffi::doca_mmap_create(null_ptr , &mut pool as *mut _)};

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(
            Self {
                pool: pool
            }
        )
    }

    /// Return the `mmap` member
    pub fn inner(&self) -> *mut ffi::doca_mmap {
        self.pool
    }

    /// Creates a memory map object representing memory ranges in remote system memory space.
    /// 
    /// Return values
    /// DOCA_SUCCESS - in case of success. doca_error code - in case of failure:
    /// DOCA_ERROR_INVALID_VALUE - if an invalid input had been received or internal error. The following errors are internal and will occur if failed to produce new mmap from export descriptor:
    /// DOCA_ERROR_NO_MEMORY - if internal memory allocation failed.
    /// DOCA_ERROR_NOT_SUPPORTED - device missing create from export capability.
    /// DOCA_ERROR_NOT_PERMITTED
    /// DOCA_ERROR_DRIVER
    pub fn new_from_export(desc_buffer: *mut c_void, desc_len: usize, dev: &DevContext) -> Result<Self, doca_error::Type> {
        let mut pool: *mut ffi::doca_mmap = std::ptr::null_mut();
        // currently we don't use any user data
        let null_ptr: *mut ffi::doca_data = std::ptr::null_mut();

        let ret = unsafe { ffi::doca_mmap_create_from_export(null_ptr, desc_buffer, desc_len, dev.ctx(), &mut pool as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(
            Self { 
                pool: pool
            }
        )
    }

    /// Compose memory map representation for later import with doca_mmap_create_from_export() for one 
    /// of the devices previously added to the memory map.
    pub fn export(&self, dev: &DevContext) -> Result<(*mut c_void, usize), doca_error::Type> {
        let len: usize = 0;
        let len_ptr = &len as *const usize as *mut usize;

        let mut export_desc: *mut c_void = std::ptr::null_mut();

        let ret = unsafe { ffi::doca_mmap_export(self.pool, dev.ctx(), &mut export_desc as *mut _, len_ptr) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok((export_desc, len))
    }
    

    /// start the DOCA mmap
    pub fn start(&self) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_mmap_start(self.pool) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// stop the DOCA mmap
    pub fn stop(&self) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_mmap_stop(self.pool) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Register DOCA memory map on a given device.
    pub fn add_device(&self, dev: &DevContext) -> Result<(), doca_error::Type> {

        let ret = unsafe { ffi::doca_mmap_dev_add(self.pool, dev.ctx()) };
        
        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Deregister given device from DOCA memory map.
    /// You should call it before free the Memory Pool.
    pub fn rm_device(&self, dev: &DevContext) -> Result<(), doca_error::Type> {

        let ret = unsafe { ffi::doca_mmap_dev_rm(self.pool, dev.ctx()) };
        
        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Add memory range to DOCA memory map.
    pub fn populate(&self, addr: *mut c_void, len: usize) -> Result<(), doca_error::Type> {
        let null_opaque: *mut c_void = std::ptr::null_mut::<c_void>();
        let ret = unsafe { doca_mmap_populate(self.pool, addr, len, page_size::get(), None, null_opaque) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Set a new max number of chunks to populate in a DOCA Memory Map. 
    /// Note: once a memory map object has been first started this functionality will not be available.
    pub fn set_max_chunks(&self, num: u32) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_mmap_set_max_num_chunks(self.pool, num) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        } 

        Ok(())
    }

}

/// The DOCA Buffer is used for reference data. 
/// It holds the information on a memory region that belongs to a DOCA memory map, 
/// and its descriptor is allocated from DOCA Buffer Inventory.
/// 
/// Notice that you should free the buffer use function `free` explicitly.
pub struct Buffer {
    inner: *mut ffi::doca_buf
}

impl Buffer {
    /// Get the buffer's data.
    pub fn get_data(&self) -> Result<*mut c_void, doca_error::Type> {
        let mut data: *mut c_void = std::ptr::null_mut();

        let ret = unsafe { ffi::doca_buf_get_data(self.inner, &mut data as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(data)
    }

    /// Set data pointer and data length
    pub fn set_data(&self, data: *mut c_void, len: usize) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_buf_set_data(self.inner, data, len) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Free the buffer to prevent memory leak,
    /// It also means that the buffer is no longer used.
    pub fn free(&self) -> Result<(), doca_error::Type> {
        let ret = unsafe { ffi::doca_buf_refcount_rm(self.inner, std::ptr::null_mut()) };
        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Return the pointer
    pub fn inner(&self) -> *mut ffi::doca_buf {
        self.inner
    }

}

/// The DOCA buffer inventory manages a pool of doca_buf objects. 
/// Each buffer obtained from an inventory is a descriptor that points to a memory region from a doca_mmap memory range of the user's choice.
pub struct BufferInventory {
    inner: *mut ffi::doca_buf_inventory,
}

impl Drop for BufferInventory {
    fn drop(&mut self) {
        unsafe { ffi::doca_buf_inventory_destroy(self.inner) };
    }
}

impl BufferInventory {
    /// Allocates buffer inventory with default/unset attributes.
    pub fn new(num: usize) -> Result<Self, doca_error::Type> {
        // currently we don't use `user_data` field
        let mut buf_inv: *mut ffi::doca_buf_inventory = std::ptr::null_mut();
        // DOCA_BUF_EXTENSION_NONE = 0;
        let ret = unsafe { ffi::doca_buf_inventory_create(std::ptr::null(), num, 0, &mut buf_inv as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        } 

        Ok(Self {
            inner: buf_inv
        })
    }

    /// Allocate single element from buffer inventory and point it to the buffer defined by `addr` & `len` arguments.
    pub fn alloc_buffer(&self, mmap: &MemoryPool, addr: *mut c_void, len: usize) -> Result<Buffer, doca_error::Type> {
        let mut buffer: *mut ffi::doca_buf = std::ptr::null_mut();
        let ret = unsafe { ffi::doca_buf_inventory_buf_by_args(self.inner, mmap.inner(), addr, len, addr, 0, &mut buffer as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        } 

        Ok(Buffer { inner: buffer })
    }

    /// Return the pointer
    pub fn inner(&self) -> *mut ffi::doca_buf_inventory {
        self.inner
    }

    /// Start element retrieval from inventory.
    pub fn start(&self) -> Result<(), doca_error::Type>{
        let ret = unsafe { ffi::doca_buf_inventory_start(self.inner) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }

    /// Stop element retrieval from inventory.
    pub fn stop(&self) -> Result<(), doca_error::Type>{
        let ret = unsafe { ffi::doca_buf_inventory_stop(self.inner) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(())
    }
}