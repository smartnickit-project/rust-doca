#![feature(get_mut_unchecked)]

use clap::{arg, App, AppSettings};
use doca::*;

use std::sync::Arc;

fn main() {
    let matches = App::new("doca dma local copy")
        .version("0.1")
        .author("Yuhan Yang")
        .about("The doca dma local copy samples on DPU")
        .setting(AppSettings::AllArgsOverrideSelf)
        .args(&[
            arg!(--pci <DEV_PCI> "DOCA DMA Device PCI address"),
            arg!(--txt [COPY_TEXT] "The text to be delivered"),
        ])
        .get_matches();

    let pci_addr = matches.value_of("pci").unwrap_or("03:00.0");
    let cpy_txt = matches
        .value_of("txt")
        .unwrap_or("This is a sample copy text");

    let length = cpy_txt.as_bytes().len();

    println!(
        "[Init] params check, pci: {}, cpy_txt {}, length {}",
        pci_addr, cpy_txt, length
    );

    // first malloc the destination buffer
    #[allow(unused_mut)]
    let mut dst_buffer = vec![0u8; length].into_boxed_slice();
    let mut src_buffer = vec![0u8; length].into_boxed_slice();

    // copy the text into src_buffer
    src_buffer.copy_from_slice(cpy_txt.as_bytes());
    println!(
        "[Before] src_buffer and dst_buffer check: {} || {}",
        String::from_utf8(src_buffer.to_vec()).unwrap(),
        String::from_utf8(dst_buffer.to_vec()).unwrap()
    );

    /* ********** The main test body ********** */

    // Create a DMA_ENGINE;
    let device = crate::open_device_with_pci(pci_addr).unwrap();

    let ctx = DMAEngine::new()
        .unwrap()
        .create_context(vec![device.clone()])
        .unwrap();

    let mut workq = DOCAWorkQueue::new(1, &ctx).unwrap();

    let mut doca_mmap = Arc::new(DOCAMmap::new().unwrap());
    unsafe { Arc::get_mut_unchecked(&mut doca_mmap).add_device(&device).unwrap() };

    let inv = BufferInventory::new(1024).unwrap();
    let mut dma_src_buf =
        DOCARegisteredMemory::new(&doca_mmap, unsafe { RawPointer::from_box(&src_buffer) })
            .unwrap()
            .to_buffer(&inv)
            .unwrap();
    unsafe { dma_src_buf.set_data(0, length).unwrap() };

    let dma_dst_buf =
        DOCARegisteredMemory::new(&doca_mmap, unsafe { RawPointer::from_box(&dst_buffer) })
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
        "[After] src_buffer and dst_buffer check: {} || {}",
        String::from_utf8(src_buffer.to_vec()).unwrap(),
        String::from_utf8(dst_buffer.to_vec()).unwrap()
    );

}
