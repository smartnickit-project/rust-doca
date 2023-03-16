//! A wrapper of the DOCA API to simplify usage in rust.
//!

use ffi::doca_error::*;
use ffi::doca_error;
use ffi::doca_error_t;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};

pub use buffer::{BufferInventory, RawPointer, DOCABuffer};
pub use device::{devices, DevContext, Device, DeviceList, open_device_with_pci};
pub use memory::DOCAMmap;
pub use registered_memory::DOCARegisteredMemory;
pub use dma::{DMAEngine, DOCAWorkQueue, DOCAEvent};

pub mod buffer;
pub mod device;
pub mod memory;
pub mod registered_memory;
pub mod dma;

pub type DOCAError = doca_error;

// pub mod context;

/// Helper function that load the exported descriptor file
/// and buffer information file into Memory, so that users
/// can use them to create a remote memory pool using
/// function `create_from_export`
///
/// # Examples
///
/// ``` ignore
/// doca::save_config_info_into_buffers(export_file, buffer_file, &mut export_desc_buffer, &mut desc_len, &mut remote_addr, &mut remote_len);
/// let remote_mmap = doca::memory::MemoryPool::new_from_export(export_desc_buffer.as_mut_ptr() as _, desc_len, &device).unwrap();
/// ```
pub fn save_config_info_into_buffers(
    export_desc_file_path: &str,
    buffer_info_file_path: &str,
    export_desc: &mut [u8],
    export_desc_len: &mut usize,
    remote_addr: &mut *mut u8,
    remote_addr_len: &mut usize,
) -> doca_error_t {
    let export_desc_file = File::open(export_desc_file_path).unwrap();
    let export_desc_file_size = export_desc_file.metadata().unwrap().len() as usize;

    // check export desc file
    *export_desc_len = export_desc_file_size;
    let mut export_desc_reader = BufReader::new(export_desc_file);

    export_desc_reader
        .read_exact(&mut export_desc[..export_desc_file_size])
        .unwrap();

    // check buffer info file
    let buffer_info_file = File::open(buffer_info_file_path).unwrap();
    let mut buffer_info_reader = BufReader::new(buffer_info_file);

    let mut remote_addr_buf = String::new();
    buffer_info_reader.read_line(&mut remote_addr_buf).unwrap();

    let remote_addr_usize: u64 = remote_addr_buf.trim().parse().unwrap();
    *remote_addr = remote_addr_usize as *mut u8;
    let mut remote_addr_len_buf = String::new();

    buffer_info_reader
        .read_line(&mut remote_addr_len_buf)
        .unwrap();
    *remote_addr_len = remote_addr_len_buf.trim().parse().unwrap();

    DOCA_SUCCESS
}

/// Helper function that export the local mmap's metadata
/// into a file so the user can transfer it to another side
/// (like from the host to DPU)
///
/// # Examples
///
/// ``` ignore
/// let export = mmap.export(device).unwrap();
/// let export_slice: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(export.0 as *mut u8, export.1)};
/// doca::save_config_info_into_files(export_slice, &src_buffer, length, export_file, buffer_file);
/// ```
pub fn save_config_info_into_files(
    export_desc: &mut [u8],
    src_buffer: &[u8],
    src_buffer_len: usize,
    export_desc_file_path: &str,
    buffer_info_file_path: &str,
) -> doca_error_t {
    // Write export descriptor into file
    let mut export_desc_file = File::create(export_desc_file_path).unwrap();

    export_desc_file.write_all(export_desc).unwrap();
    export_desc_file.flush().unwrap();

    // Write local buffer info into file
    let mut buffer_info_file = File::create(buffer_info_file_path).unwrap();

    writeln!(buffer_info_file, "{}", src_buffer.as_ptr() as u64).unwrap();
    writeln!(buffer_info_file, "{}", src_buffer_len).unwrap();
    buffer_info_file.flush().unwrap();

    DOCA_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindgen_test_save_config() {
        let mut desc_string = String::from("Hello!");
        let src_buffer_string = String::from("1234567890");

        let src_buffer = src_buffer_string.as_bytes();
        unsafe {
            save_config_info_into_files(
                desc_string.as_bytes_mut(),
                src_buffer,
                src_buffer.len(),
                "/tmp/desc.txt",
                "/tmp/buffer.txt",
            )
        };

        // Then read the config into buffer
        let mut export_desc_len: usize = 0;
        let mut remote_addr_len: usize = 0;

        let mut remote_addr: *mut u8 = std::ptr::null_mut();

        let buffer = Box::new([0u8; 1024]);
        let buffer_ref = Box::leak(buffer);
        save_config_info_into_buffers(
            "/tmp/desc.txt",
            "/tmp/buffer.txt",
            buffer_ref,
            &mut export_desc_len,
            &mut remote_addr,
            &mut remote_addr_len,
        );

        // alright check all these
        assert_eq!(remote_addr_len, src_buffer.len());
        unsafe { assert_eq!(export_desc_len, desc_string.as_bytes_mut().len()) };
        assert_eq!(
            String::from_utf8(buffer_ref[..export_desc_len].to_vec()).unwrap(),
            String::from("Hello!")
        );
        assert_eq!(remote_addr as u64, src_buffer.as_ptr() as u64);
    }
}
