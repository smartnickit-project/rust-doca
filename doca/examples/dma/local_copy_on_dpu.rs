use clap::{arg, App, AppSettings};
use doca::{dma::*, context::Context};
use ffi::doca_error::DOCA_SUCCESS;

fn main() {
    let matches = App::new("doca dma local copy")
        .version("0.1")
        .author("Yuhan Yang")
        .about("The doca dma local copy samples on DPU")
        .setting(AppSettings::AllArgsOverrideSelf)
        .args(
            &[
                arg!(--pci <DEV_PCI> "DOCA DMA Device PCI address"),
                arg!(--txt [COPY_TEXT] "The text to be delivered")
            ]
        ).get_matches();

    let pci_addr = matches.value_of("pci").unwrap_or("03:00.0");
    let cpy_txt = matches.value_of("txt").unwrap_or("This is a sample copy text");

    let length = cpy_txt.as_bytes().len();

    println!("params check, pci: {}, cpy_txt {}, length {}", pci_addr, cpy_txt, length);


    // first malloc the destination buffer & destination buffer
    let mut dst_buffer = vec![0u8; length].into_boxed_slice();
    let mut src_buffer = vec![0u8; length].into_boxed_slice();

    // copy the text into src_buffer
    src_buffer.copy_from_slice(cpy_txt.as_bytes());
    let str = String::from_utf8(src_buffer.to_vec()).unwrap();
    println!("src_buffer check: {}", str);

    // Create a DMA_ENGINE;
    let dma_engine = DMAEngine::new().unwrap();

    // Create a doca context
    let ctx = Context::new(&dma_engine);

    // Open a doca device
    let device = doca::device::open_device_with_pci(pci_addr).unwrap();

    // Create mmap & buf repository
    let mmap = doca::memory::MemoryPool::new().unwrap();
    let buf_inv = doca::memory::BufferInventory::new(2).unwrap();

    // Set attribute & start the struct
    mmap.set_max_chunks(2).unwrap();
    mmap.start().unwrap();
    mmap.add_device(&device).unwrap();

    buf_inv.start().unwrap();

    // Add a device into the DOCA context & start the context
    assert_eq!(ctx.add_device(&device), DOCA_SUCCESS);
    assert_eq!(ctx.start(), DOCA_SUCCESS);

    // Create & Bind the workQ
    let workq = doca::context::WorkQueue::new(32).unwrap();
    ctx.add_workq(&workq).unwrap();

    // Register the memory
    mmap.populate(dst_buffer.as_mut_ptr() as _, length).unwrap();
    mmap.populate(src_buffer.as_mut_ptr() as _, length).unwrap();

    // Allocate Buffer
    let dst_doca_buf = buf_inv.alloc_buffer(&mmap, dst_buffer.as_mut_ptr() as _, length).unwrap();
    let src_doca_buf = buf_inv.alloc_buffer(&mmap, src_buffer.as_mut_ptr() as _, length).unwrap();

    // construct the dma job
    let mut dma_job = doca::dma::DMAJob::new();
    dma_job.set_type();
    dma_job.set_flags();
    dma_job.set_ctx(&ctx);
    dma_job.set_dst(&dst_doca_buf);
    dma_job.set_src(&src_doca_buf);

    src_doca_buf.set_data(src_buffer.as_mut_ptr() as _, length).unwrap();

    // enqueue the job
    workq.submit(&dma_job).unwrap();

    // wait for completion
    let mut event = doca::context::Event::new();
    let ret = workq.poll_completion(&mut event);
    if ret != DOCA_SUCCESS {
        println!("Failed to poll the job!!");
        return;
    }

    // check status
    let result = event.result() as u32;
    if result != DOCA_SUCCESS {
        println!("Job failed!");
        return;
    }

    println!("dma copy success, the information in dst buffer: {}", String::from_utf8(dst_buffer.to_vec()).unwrap());

    // Clean resources
    ctx.rm_workq(&workq).unwrap();
    dst_doca_buf.free().unwrap();
    src_doca_buf.free().unwrap();

    mmap.rm_device(&device).unwrap();
    ctx.stop();
    ctx.rm_device(&device);

    // The rest leaves for `Drop` Trait.

}
