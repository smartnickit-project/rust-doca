//! A wrapper of the DOCA API to simplify usage in rust.
//! It also provides automatic lifecycle management over `Arc`.
//!
//! Note that the drop order between these structs should satisfy that
//! - [`DOCABuffer`] should be dropped before the [`BufferInventory`]
//! - [`DOCAContext`] should be dropped before the [`DevContext`] added into it
//! - [`DOCAWorkQueue`] should be dropped before the [`DOCAContext`]
//! - [`DOCAContext`] should be dropped before its original Engine dropped
//! - [`DOCAMmap`] should be dropped before the [`DevContext`] registered into it
//!
//! - The [`context`] module contains wrapper of the execution
//! model in DOCA, including a submodule [`work_queue`].
//!
//! - The [`device`] module provides wrapper for
//! managing DOCA devices.
//!
//! - The [`memory`] module provides wrapper for DOCA memory
//! subsystem, including [`doca_buffer`] and [`doca_mmap`].
//!
//! - The [`dma`] module provides wrapper for DOCA DMA engine,
//! which provides the ability to copy data between memory
//! using hardware acceleration.
//!
//!
//!
#![deny(
    missing_docs,
    unused_imports,
    unused_must_use,
    unused_parens,
    unused_qualifications
)]

use ffi::doca_error;
use std::ffi::c_void;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::ptr::NonNull;
use std::slice;

pub use device::{devices, open_device_with_pci, DevContext, Device, DeviceList};
pub use dma::{DMAEngine, DOCAEvent, DOCAWorkQueue};
pub use memory::buffer::{BufferInventory, DOCABuffer, RawPointer};
pub use memory::registered_memory::DOCARegisteredMemory;
pub use memory::DOCAMmap;

pub mod context;
pub mod device;
pub mod dma;
pub mod memory;

/// Error type
pub type DOCAError = doca_error;

/// Result type
pub type DOCAResult<T> = Result<T, DOCAError>;

// FIXME: Not sure about max length of the exported information
const DOCA_MAX_EXPORT_LENGTH: usize = 2048;

/// Struct used for the return value for function `load_config`
pub struct LoadedInfo {
    /// The metadata for the remote mmap
    pub export_desc: RawPointer,
    /// The remote address for the mmap
    // TODO: support multiple remote address transfer
    pub remote_addr: RawPointer,
}

/// Helper function that load the exported descriptor file
/// and buffer information file into Memory, so that users
/// can use them to create a remote memory map object and
/// transfer data.
///
/// # Examples
///
/// ``` rust, no_run
/// use doca::DOCAMmap;
/// 
/// // Create the device according to the pci address
/// let device = doca::device::open_device_with_pci("17:00.0").unwrap();
/// 
/// // Load the config from the files and create the remote memory map object
/// let remote_configs = doca::load_config("/tmp/export.txt", "/tmp/buffer.txt");
/// let mut remote_mmap = DOCAMmap::new_from_export(remote_configs.export_desc, &device).unwrap();
/// ```
pub fn load_config(export_desc_file_path: &str, buffer_info_file_path: &str) -> LoadedInfo {
    let export_desc_file = File::open(export_desc_file_path).unwrap();
    let export_desc_file_size = export_desc_file.metadata().unwrap().len() as usize;

    let mut export_desc_buffer = vec![0u8; DOCA_MAX_EXPORT_LENGTH].into_boxed_slice();

    // check export description
    let mut export_desc_reader = BufReader::new(export_desc_file);

    export_desc_reader
        .read_exact(&mut export_desc_buffer[..export_desc_file_size])
        .unwrap();

    // check remote memory region's info
    let buffer_info_file = File::open(buffer_info_file_path).unwrap();
    let mut buffer_info_reader = BufReader::new(buffer_info_file);

    let mut remote_addr_buf = String::new();
    buffer_info_reader.read_line(&mut remote_addr_buf).unwrap();

    let remote_addr_usize: u64 = remote_addr_buf.trim().parse().unwrap();
    let remote_addr = remote_addr_usize as *mut c_void;
    let mut remote_addr_len_buf = String::new();

    buffer_info_reader
        .read_line(&mut remote_addr_len_buf)
        .unwrap();
    let remote_addr_len: usize = remote_addr_len_buf.trim().parse().unwrap();

    LoadedInfo {
        export_desc: RawPointer {
            // use the clone to keep the boxed memory keep alive even the function ends.
            // The memory could be dropped after the program ends automatically.
            inner: NonNull::new(Box::into_raw(export_desc_buffer) as *mut _).unwrap(),
            payload: export_desc_file_size,
        },
        remote_addr: RawPointer {
            inner: NonNull::new(remote_addr).unwrap(),
            payload: remote_addr_len,
        },
    }
}

/// Helper function that export the local mmap's metadata
/// into a file so the user can transfer it to another side
///
/// # Examples
///
/// ``` rust, no_run
/// use doca::DOCAMmap;
/// use doca::RawPointer;
/// use std::ptr::NonNull;
/// 
/// // allocate the buffer
/// let mut src_buffer = vec![0u8; 1024].into_boxed_slice();
/// 
/// let src_raw = RawPointer {
///     inner: NonNull::new(src_buffer.as_mut_ptr() as *mut _).unwrap(),
///     payload: 1024,
/// };
/// 
/// // Create the memory map object and add device into it.
/// let mut local_mmap =DOCAMmap::new().unwrap();
/// let device = doca::device::open_device_with_pci("17:00.0").unwrap();
/// let dev_idx = local_mmap.add_device(&device).unwrap();
/// 
/// // populate the buffer into the mmap
/// local_mmap.populate(src_raw).unwrap();
/// 
/// // Generate the exported information and save it into files
/// let export = local_mmap.export(dev_idx).unwrap();
/// doca::save_config(export, src_raw, "/tmp/export.txt", "/tmp/buffer.txt");
/// ```
pub fn save_config(
    export_desc: RawPointer,
    src_buffer: RawPointer,
    export_desc_file_path: &str,
    buffer_info_file_path: &str,
) {
    // Write export descriptor into file
    let mut export_desc_file = File::create(export_desc_file_path).unwrap();

    let export_slice = unsafe {
        slice::from_raw_parts_mut(export_desc.inner.as_ptr() as *mut u8, export_desc.payload)
    };

    export_desc_file.write_all(export_slice).unwrap();
    export_desc_file.flush().unwrap();

    // Write local buffer info into file
    let mut buffer_info_file = File::create(buffer_info_file_path).unwrap();

    writeln!(buffer_info_file, "{}", src_buffer.inner.as_ptr() as u64).unwrap();
    writeln!(buffer_info_file, "{}", src_buffer.payload).unwrap();
    buffer_info_file.flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindgen_test_save_config() {
        let mut desc_string = String::from("Hello!");
        let mut src_buffer_string = String::from("1234567890");

        let desc_raw = RawPointer {
            inner: NonNull::new(desc_string.as_mut_ptr() as *mut _).unwrap(),
            payload: desc_string.as_bytes().len(),
        };

        let src_raw = RawPointer {
            inner: NonNull::new(src_buffer_string.as_mut_ptr() as *mut _).unwrap(),
            payload: src_buffer_string.as_bytes().len(),
        };

        let src_buffer = src_buffer_string.as_bytes();
        save_config(
            desc_raw,
            src_raw,
            "/tmp/desc_test.txt",
            "/tmp/buffer_test.txt",
        );

        let configs = load_config("/tmp/desc_test.txt", "/tmp/buffer_test.txt");

        // alright check all these
        assert_eq!(configs.remote_addr.payload, src_buffer.len());
        unsafe {
            assert_eq!(
                configs.export_desc.payload,
                desc_string.as_bytes_mut().len()
            )
        };
        unsafe {
            assert_eq!(
                String::from_utf8(
                    slice::from_raw_parts(
                        configs.export_desc.inner.as_ptr() as *mut u8,
                        configs.export_desc.payload
                    )
                    .to_vec()
                )
                .unwrap(),
                String::from("Hello!")
            )
        };
        assert_eq!(
            configs.remote_addr.inner.as_ptr() as u64,
            src_buffer.as_ptr() as u64
        );
    }
}
