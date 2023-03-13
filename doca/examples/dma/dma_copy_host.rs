use clap::{arg, App, AppSettings};
extern crate ctrlc;

fn main() {
    let matches = App::new("doca dma copy")
        .version("0.1")
        .author("Yuhan Yang")
        .about("The doca dma copy samples on Host Side")
        .setting(AppSettings::AllArgsOverrideSelf)
        .args(
            &[
                arg!(--pci <DEV_PCI> "DOCA DMA Device PCI address"),
                arg!(--txt [COPY_TEXT] "The text to be delivered"),
                arg!(--export <FILE_PATH> "export descriptor file path"),
                arg!(--buffer <FILE_PATH> "buffer info file path")
            ]
        ).get_matches();

    // get parameters
    let pci_addr = matches.value_of("pci").unwrap_or("17:00.0");
    let cpy_txt = matches.value_of("txt").unwrap_or("This is a sample copy text");
    let export_file = matches.value_of("export").unwrap_or("/tmp/export.txt");
    let buffer_file = matches.value_of("buffer").unwrap_or("/tmp/buffer.txt");

    let length = cpy_txt.as_bytes().len();

    println!("params check, pci: {}, cpy_txt {}, length {}", pci_addr, cpy_txt, length);

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));

    // first malloc the destination buffer & destination buffer
    let mut src_buffer = vec![0u8; length].into_boxed_slice();

    // copy the text into src_buffer
    src_buffer.copy_from_slice(cpy_txt.as_bytes());
    let str = String::from_utf8(src_buffer.to_vec()).unwrap();
    println!("src_buffer check: {}", str);

    // Open device
    let device = doca::device::open_device_with_pci(pci_addr).unwrap();

    // Create mmap & init
    let mmap = doca::memory::MemoryPool::new().unwrap();

    mmap.start().unwrap();
    mmap.add_device(&device).unwrap();

    // populate the buffer into the mmap
    mmap.populate(src_buffer.as_mut_ptr() as _, length).unwrap();

    // and export it into memory so later we can store it into a file
    let export = mmap.export(&device).unwrap();

    let export_slice: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(export.0 as *mut u8, export.1)};

    doca::save_config_info_into_files(export_slice, &src_buffer, length, export_file, buffer_file);
    println!("Please copy {} and {} to the DPU and run DMA Copy DPU sample before closing", export_file, buffer_file);


    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // Your program's code goes here
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }

    println!("Server is down!");

    mmap.rm_device(&device).unwrap();

}
