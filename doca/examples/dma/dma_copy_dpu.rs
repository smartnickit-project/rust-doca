extern crate ctrlc;
use clap::{arg, App, AppSettings};
use ffi::doca_error::DOCA_SUCCESS;


fn main() {
    let matches = App::new("doca dma copy")
        .version("0.1")
        .author("Yuhan Yang")
        .about("The doca dma copy samples on DPU")
        .setting(AppSettings::AllArgsOverrideSelf)
        .args(
            &[
                arg!(--pci <DEV_PCI> "DOCA DMA Device PCI address"),
                arg!(--export <FILE_PATH> "export descriptor file path"),
                arg!(--buffer <FILE_PATH> "buffer info file path")
            ]
        ).get_matches();

    let pci_addr = matches.value_of("pci").unwrap_or("17:00.0");
    let export_file = matches.value_of("export").unwrap_or("/tmp/export.txt");
    let buffer_file = matches.value_of("buffer").unwrap_or("/tmp/buffer.txt");

    // Create a DMA_ENGINE & context based on the engine
    let engine = doca::dma::DMAEngine::new().unwrap();
    let ctx = engine.get_ctx();

    // Open device, Create mmap & buf repository
    let device = doca::device::open_device_with_pci(pci_addr).unwrap();
    let mmap = doca::memory::MemoryPool::new().unwrap();
    let buf_inv = doca::memory::BufferInventory::new(2).unwrap();

    // Set attribute & start the Memory Pool
    mmap.set_max_chunks(2).unwrap();
    mmap.start().unwrap();
    mmap.add_device(&device).unwrap();

    // Start the buffer inventory
    buf_inv.start().unwrap();

    // Add a device into the DOCA context & start the context
    assert_eq!(ctx.add_device(&device), DOCA_SUCCESS);
    assert_eq!(ctx.start(), DOCA_SUCCESS);

    // Create & Bind the workQ
    let workq = doca::context::WorkQueue::new(32).unwrap();
    ctx.add_workq(&workq).unwrap();


    let mut export_desc_buffer = vec![0u8; 1024].into_boxed_slice();
    let mut remote_len: usize = 0;
    let mut desc_len: usize = 0;
    let mut remote_addr: *mut u8 = std::ptr::null_mut();

    // Get information to construct the remote Memory Pool
    doca::save_config_info_into_buffers(export_file, buffer_file, &mut export_desc_buffer, &mut desc_len, &mut remote_addr, &mut remote_len);

    println!("Check export len {}, remote addr len {}, remote addr {:?}", desc_len, remote_len, remote_addr);

    let mut dpu_buffer = vec![0u8; remote_len].into_boxed_slice();
    mmap.populate(dpu_buffer.as_mut_ptr() as _, remote_len).unwrap();

    // Create the remote Memory Pool
    let remote_mmap = doca::memory::MemoryPool::new_from_export(export_desc_buffer.as_mut_ptr() as _, desc_len, &device).unwrap();
    let src_doca_buf = buf_inv.alloc_buffer(&remote_mmap, remote_addr as _, remote_len).unwrap();
    let dst_doca_buf = buf_inv.alloc_buffer(&mmap, dpu_buffer.as_mut_ptr() as _, remote_len).unwrap();

    // Construct the dma job
    let mut dma_job = doca::dma::DMAJob::new();
    dma_job.set_type();
    dma_job.set_flags();
    dma_job.set_ctx(&ctx);
    dma_job.set_dst(&dst_doca_buf);
    dma_job.set_src(&src_doca_buf);

    src_doca_buf.set_data(remote_addr as _, remote_len).unwrap();

    // Enqueue the job
    workq.submit(&dma_job).unwrap();

    // Wait for completion
    let mut event = doca::context::Event::new();
    let ret = workq.poll_completion(&mut event);
    if ret != DOCA_SUCCESS {
        println!("Failed to poll the job!!");
        return;
    }

    // Check status
    let result = event.result() as u32;
    if result != DOCA_SUCCESS {
        println!("Job failed!");
        return;
    }

    println!("dma copy success, the information in dst buffer: {}", String::from_utf8(dpu_buffer.to_vec()).unwrap());

    // Clean resources
    ctx.rm_workq(&workq).unwrap();
    dst_doca_buf.free().unwrap();
    src_doca_buf.free().unwrap();

    mmap.rm_device(&device).unwrap();
    ctx.stop();
    ctx.rm_device(&device);

    // The rest leaves for `Drop` Trait.

}
