//! Wrap DOCA Device into rust struct.
//! With the help of the wrapper, creating, managing and querying
//! the device is extremely simple.
//! Note that we also use `Arc` to automatically manage the lifecycle of the
//! device-related data structures.
//!
//! Example usage of opening a device context with a given device name:
//!
//! ```
//! use doca::open_device_with_pci;
//! let device_ctx = open_device_with_pci("03:00.0");
//!
//!
//! ```
//!
//! or
//!
//! ```
//! use doca::devices;
//! let device_ctx = devices().unwrap().get(0).unwrap().open().unwrap();
//! ```
//!

use ffi::doca_error;
use std::{ptr::NonNull, sync::Arc};

/// DOCA Device list
pub struct DeviceList(&'static mut [*mut ffi::doca_devinfo]);

unsafe impl Sync for DeviceList {}
unsafe impl Send for DeviceList {}

impl Drop for DeviceList {
    fn drop(&mut self) {
        unsafe { ffi::doca_devinfo_list_destroy(self.0.as_mut_ptr()) };

        // Show drop order only in `debug` mode
        #[cfg(debug_assertions)]
        println!("DeviceList is dropped!");
    }
}

/// Get list of all available local devices.
///
/// # Errors
///
///  - `DOCA_ERROR_INVALID_VALUE`: received invalid input.
///  - `DOCA_ERROR_NO_MEMORY`: failed to allocate enough space.
///  - `DOCA_ERROR_NOT_FOUND`: failed to get RDMA devices list
///
pub fn devices() -> Result<Arc<DeviceList>, doca_error> {
    let mut n = 0u32;
    let mut dev_list: *mut *mut ffi::doca_devinfo = std::ptr::null_mut();
    let ret = unsafe { ffi::doca_devinfo_list_create(&mut dev_list as *mut _, &mut n as *mut _) };

    if dev_list.is_null() || ret != doca_error::DOCA_SUCCESS {
        return Err(ret);
    }

    let devices = unsafe { std::slice::from_raw_parts_mut(dev_list, n as usize) };

    Ok(Arc::new(DeviceList(devices)))
}

impl DeviceList {
    /// Returns the number of devices.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are any devices.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of devices.
    pub fn num_devices(&self) -> usize {
        self.len()
    }

    /// Returns the device at the given `index`, or `None` if out of bounds.
    pub fn get(self: &Arc<Self>, index: usize) -> Option<Arc<Device>> {
        self.0.get(index).map(|d| {
            Arc::new(Device {
                inner: NonNull::new(*d).unwrap(),
                parent_devlist: self.clone(),
            })
        })
    }
}

/// An DOCA device
pub struct Device {
    inner: NonNull<ffi::doca_devinfo>,

    // a device hold to ensure the device list is not freed
    // before the Device is freed
    #[allow(dead_code)]
    parent_devlist: Arc<DeviceList>,
}

unsafe impl Sync for Device {}
unsafe impl Send for Device {}

impl Device {
    /// Return the PCIe address of the doca device, e.g "17:00.1".
    /// The matching between the str & `doca_pci_bdf` can be seen
    /// as below.
    /// ---------------------------------------
    /// -- 4 -- b -- : -- 0 -- 0 -- . -- 1 ----
    /// --   BUS     |    DEVICE    | FUNCTION
    ///
    /// # Errors
    ///
    ///  - `DOCA_ERROR_INVALID_VALUE`: received invalid input.
    ///
    pub fn name(&self) -> Result<String, doca_error> {
        let mut pci_bdf: ffi::doca_pci_bdf = Default::default();
        let ret =
            unsafe { ffi::doca_devinfo_get_pci_addr(self.inner_ptr(), &mut pci_bdf as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        // first check the `bus` part
        let bus = unsafe { pci_bdf.__bindgen_anon_1.__bindgen_anon_1.bus() };
        let device = unsafe { pci_bdf.__bindgen_anon_1.__bindgen_anon_1.device() };
        let func = unsafe { pci_bdf.__bindgen_anon_1.__bindgen_anon_1.function() };

        Ok(format!(
            "{:x}{:x}:{:x}{:x}.{:x}",
            bus / 16,
            bus % 16,
            device / 16,
            device % 16,
            func
        ))
    }

    /// Open a DOCA device and store it as a context for further use.
    pub fn open(self: &Arc<Self>) -> Result<Arc<DevContext>, doca_error> {
        DevContext::with_device(self.clone())
    }

    /// Get the maximum supported buffer size for DMA job.
    pub fn get_max_buf_size(&self) -> Result<u64, doca_error> {
        let mut num: u64 = 0;
        let ret = unsafe { ffi::doca_dma_get_max_buf_size(self.inner_ptr(), &mut num as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(num)
    }

    /// Return the device
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_devinfo {
        self.inner.as_ptr()
    }
}

/// An opened Doca Device
pub struct DevContext {
    ctx: NonNull<ffi::doca_dev>,
    #[allow(dead_code)]
    parent: Arc<Device>,
}

impl Drop for DevContext {
    fn drop(&mut self) {
        unsafe { ffi::doca_dev_close(self.ctx.as_ptr()) };

        // Show drop order only in `debug` mode
        #[cfg(debug_assertions)]
        println!("Device Context is dropped!");
    }
}

impl DevContext {
    /// Opens a context for the given device, so we can use it later.
    pub fn with_device(dev: Arc<Device>) -> Result<Arc<DevContext>, doca_error> {
        let mut ctx: *mut ffi::doca_dev = std::ptr::null_mut();
        let ret = unsafe { ffi::doca_dev_open(dev.inner_ptr(), &mut ctx as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(Arc::new(DevContext {
            ctx: NonNull::new(ctx).ok_or(doca_error::DOCA_ERROR_INVALID_VALUE)?,
            parent: dev,
        }))
    }

    /// Return the DOCA Device context raw pointer
    #[inline]
    pub unsafe fn inner_ptr(&self) -> *mut ffi::doca_dev {
        self.ctx.as_ptr()
    }
}

/// Open a DOCA Device with the given PCI address
///
/// Examples
/// ```
/// use doca::open_device_with_pci;
/// let device = open_device_with_pci("03:00.0");
/// ```
///
pub fn open_device_with_pci(pci: &str) -> Result<Arc<DevContext>, doca_error> {
    let dev_list = devices()?;

    for i in 0..dev_list.num_devices() {
        let device = dev_list.get(i).unwrap();
        let pci_addr = device.name()?;
        if pci_addr.eq(pci) {
            // open the device
            return device.open();
        }
    }

    Err(doca_error::DOCA_ERROR_INVALID_VALUE)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_device_and_check() {
        let ret = crate::device::devices();
        assert!(ret.is_ok());

        let devices = ret.unwrap();

        // specially, there're 4 local doca devices on `pro0`
        // which has been checked by the original C program
        println!("len: {}", devices.len());
        assert_ne!(devices.len(), 0);

        for i in 0..devices.num_devices() {
            let device = devices.get(i).unwrap();
            let pci_addr = device.name().unwrap();
            println!("device pci addr {}", pci_addr);
        }
    }

    #[test]
    fn test_get_and_open_a_device() {
        let device = crate::device::devices().unwrap().get(0).unwrap().open();
        assert!(device.is_ok());
    }

    #[test]
    fn test_dev_max_buf() {
        let device = crate::device::devices().unwrap().get(0).unwrap();
        let ret = device.get_max_buf_size();
        assert!(ret.is_ok());
        println!("max buf size: {}", ret.unwrap());
    }
}
