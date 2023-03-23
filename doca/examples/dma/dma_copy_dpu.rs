#![feature(get_mut_unchecked)]

use clap::{arg, App, AppSettings};
use doca::{dma::DOCAContext, *};

use std::sync::Arc;

fn main() {
    let matches = App::new("doca remote copy")
        .version("0.1")
        .author("Yuhan Yang")
        .about("The doca dma remote copy samples on DPU Side")
        .setting(AppSettings::AllArgsOverrideSelf)
        .args(&[
            arg!(--pci <DEV_PCI> "DOCA DMA Device PCI address"),
            arg!(--export [FILE_PATH] "export descriptor file path"),
            arg!(--buffer [FILE_PATH] "buffer info file path"),
        ])
        .get_matches();

    let pci_addr = matches.value_of("pci").unwrap_or("03:00.0");
    let export_file = matches.value_of("export").unwrap_or("/tmp/export.txt");
    let buffer_file = matches.value_of("buffer").unwrap_or("/tmp/buffer.txt");

    // Get information to construct the remote Memory Pool
    let remote_configs = doca::load_config(export_file, buffer_file);

    println!(
        "Check export len {}, remote len {}, remote addr {:?}",
        remote_configs.export_desc.payload,
        remote_configs.remote_addr.payload,
        remote_configs.remote_addr.inner.as_ptr()
    );

    // Allocate the local buffer to store the transferred data
    #[allow(unused_mut)]
    let mut dpu_buffer = vec![0u8; remote_configs.remote_addr.payload].into_boxed_slice();

    /* ********** The main test body ********** */

    // Create a DMA_ENGINE;
    let device = crate::open_device_with_pci(pci_addr).unwrap();

    let dma = DMAEngine::new().unwrap();

    let ctx = DOCAContext::new(&dma, vec![device.clone()]).unwrap();

    let mut workq = DOCAWorkQueue::new(1, &ctx).unwrap();

    let mut doca_mmap = Arc::new(DOCAMmap::new().unwrap());
    unsafe {
        Arc::get_mut_unchecked(&mut doca_mmap)
            .add_device(&device)
            .unwrap()
    };

    // Create the remote mmap
    #[allow(unused_mut)]
    let mut remote_mmap =
        Arc::new(DOCAMmap::new_from_export(remote_configs.export_desc, &device).unwrap());

    let inv = BufferInventory::new(1024).unwrap();
    let mut dma_src_buf =
        DOCARegisteredMemory::new_from_remote(&remote_mmap, remote_configs.remote_addr)
            .unwrap()
            .to_buffer(&inv)
            .unwrap();
    unsafe {
        dma_src_buf
            .set_data(0, remote_configs.remote_addr.payload)
            .unwrap()
    };

    let dma_dst_buf =
        DOCARegisteredMemory::new(&doca_mmap, unsafe { RawPointer::from_box(&dpu_buffer) })
            .unwrap()
            .to_buffer(&inv)
            .unwrap();

    /* Start to submit the DMA job!  */
    let job = workq.create_dma_job(dma_src_buf, dma_dst_buf);
    workq.submit(&job).expect("failed to submit the job");

    loop {
        let event = workq.poll_completion();
        match event {
            Ok(_e) => {
                println!("Job finished!");
                break;
            }
            Err(e) => {
                if e == DOCAError::DOCA_ERROR_AGAIN {
                    continue;
                } else {
                    panic!("Job failed! {:?}", e);
                }
            }
        }
    }

    /* ------- Finalize check ---------- */
    println!(
        "[After] dst_buffer check: {}",
        String::from_utf8(dpu_buffer.to_vec()).unwrap()
    );
}
