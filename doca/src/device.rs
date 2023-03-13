//! DOCA Device related

use ffi::doca_error;

/// DOCA Device list
pub struct DeviceList(&'static mut [*mut ffi::doca_devinfo]);

unsafe impl Sync for DeviceList {}
unsafe impl Send for DeviceList {}

impl Drop for DeviceList {
    fn drop(&mut self) {
        unsafe { ffi::doca_devinfo_list_destroy(self.0.as_mut_ptr()) };
    }
}

/// Get list of all available local devices.
/// 
/// # Errors
///
///  - `DOCA_ERROR_INVALID_VALUE`: received invalid input.
///  - `DOCA_ERROR_NO_MEMORY`: failed to allocate enough space.
///  - `DOCA_ERROR_NOT_FOUND`: failed to get RDMA devices list
pub fn devices() -> Result<DeviceList, doca_error::Type> {
    let mut n = 0u32;
    let mut dev_list : *mut *mut ffi::doca_devinfo = std::ptr::null_mut();
    let ret = unsafe { ffi::doca_devinfo_list_create(&mut dev_list as *mut _, &mut n as *mut _) };

    if dev_list.is_null() || ret != doca_error::DOCA_SUCCESS {
        return Err(ret);
    }

    let devices = unsafe { std::slice::from_raw_parts_mut(dev_list, n as usize) };

    Ok(DeviceList(devices))
}

impl DeviceList {
    /// Returns an iterator over all found devices.
    pub fn iter(&self) -> DeviceListIter<'_> {
        DeviceListIter { list: self, i: 0 }
    }

    /// Returns the number of devices.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are any devices.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the device at the given `index`, or `None` if out of bounds.
    pub fn get(&self, index: usize) -> Option<Device<'_>> {
        self.0.get(index).map(|d| d.into())
    }
}

impl<'a> IntoIterator for &'a DeviceList {
    type Item = <DeviceListIter<'a> as Iterator>::Item;
    type IntoIter = DeviceListIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DeviceListIter { list: self, i: 0 }
    }
}

/// Iterator over a `DeviceList`.
pub struct DeviceListIter<'iter> {
    list: &'iter DeviceList,
    i: usize,
}

impl<'iter> Iterator for DeviceListIter<'iter> {
    type Item = Device<'iter>;
    fn next(&mut self) -> Option<Self::Item> {
        let e = self.list.0.get(self.i);
        if e.is_some() {
            self.i += 1;
        }
        e.map(|e| e.into())
    }
}

/// An DOCA device
pub struct Device<'devlist>(&'devlist *mut ffi::doca_devinfo);
unsafe impl<'devlist> Sync for Device<'devlist> {}
unsafe impl<'devlist> Send for Device<'devlist> {}

impl<'d> From<&'d *mut ffi::doca_devinfo> for Device<'d> {
    fn from(d: &'d *mut ffi::doca_devinfo) -> Self {
        Device(d)
    }
}

impl<'devlist> Device<'devlist> {
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
    pub fn name(&self) -> Option<String> {
        let mut pci_bdf: ffi::doca_pci_bdf = Default::default();
        let ret = unsafe { ffi::doca_devinfo_get_pci_addr(*self.0, &mut pci_bdf as *mut _) };
        
        if ret != doca_error::DOCA_SUCCESS {
            return None
        }

        // first check the `bus` part
        let bus = unsafe { pci_bdf.__bindgen_anon_1.__bindgen_anon_1.bus() };
        let device = unsafe { pci_bdf.__bindgen_anon_1.__bindgen_anon_1.device() };
        let func = unsafe { pci_bdf.__bindgen_anon_1.__bindgen_anon_1.function() };

        Some(format!("{:x}{:x}:{:x}{:x}.{:x}", bus/16, bus%16, device/16, device%16, func))
    }

    /// Open a DOCA device and store it as a context for further use.
    pub fn open(&self) -> Result<DevContext, doca_error::Type> {
        DevContext::with_device(*self.0)
    }

    /// Return the device
    pub fn inner(&self) -> *mut ffi::doca_devinfo {
        *self.0
    }
}

/// An opened Doca Device
pub struct DevContext {
    ctx: *mut ffi::doca_dev 
}

impl Drop for DevContext {
    fn drop(&mut self) {
        unsafe { ffi::doca_dev_close(self.ctx) };
    }
}

impl DevContext {
    /// Opens a context for the given device, so we can use it later.
    pub fn with_device(dev: *mut ffi::doca_devinfo) -> Result<DevContext, doca_error::Type> {
        let mut ctx: *mut ffi::doca_dev = std::ptr::null_mut();
        let ret = unsafe { ffi::doca_dev_open(dev, &mut ctx as *mut _) };

        if ret != doca_error::DOCA_SUCCESS {
            return Err(ret);
        }

        Ok(
            DevContext {
                ctx: ctx
            }
        )
    }

    /// Return the DOCA Device Context
    pub fn ctx(&self) -> *mut ffi::doca_dev {
        self.ctx
    }
}

/// Open a DOCA Device with the given PCI address
/// 
/// Examples
/// ```
/// let device = open_device_with_pci("03:00.0").unwrap();
/// ```
pub fn open_device_with_pci(pci: &str) -> Result<DevContext, doca_error::Type> {
    let dev_list = devices().unwrap();

    for device in &dev_list {
        let pci_addr = device.name().unwrap();
        if pci_addr.eq(pci) {
            // open the device
            return device.open()
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

        for device in &devices {
            let pci_addr = device.name().unwrap();
            println!("device pci addr {}", pci_addr);
        }
    }

    #[test]
    fn test_get_and_open_a_device() {
        let device = crate::device::devices().unwrap().get(0).unwrap().open();
        assert!(device.is_ok());
    }
}
