## DOCA DMA Intro
doca dma is a library based on NVIDIA DPU
which is capable to transfer data between
host and DPU(e.g. a DMA from host to host, 
host to DPU & DPU to host).
It consists of 4 components, buffer, device, dma,
and memory.

## How it works
The semantics of DOCA is actually similar to RDMA. You need
to open a device, create a context, register a memory region, and create a work queue before sending requests to it. The document will introduce the components in this section.

### Device
To use DMA, the user need to rely on one DOCA device(like an `ibv_context` in RDMA). This module provides interfaces for 
user to fetch device list, get each device's information(like its pci address), 
and open it for later usage. Also, we give the users a useful 
function `open_device_with_pci` so that they can open a doca device 
directly with the PCIe address they find by command `lspci | grep Mellanox`.

Notice that the user can find DOCA devices by command `lspci | grep Mellanox`
```bash
17:00.0 Ethernet controller: Mellanox Technologies MT42822 BlueField-2 integrated ConnectX-6 Dx network controller (rev 01)
17:00.1 Ethernet controller: Mellanox Technologies MT42822 BlueField-2 integrated ConnectX-6 Dx network controller (rev 01)
```

Here, due to the link layer of `Ethernet`, the outputs are `Ethernet controller`. If the link layer is IB, the outputs should be `Infiniband controller`.

### Memory
Memory is an important module in DOCA especially for DOCA DMA. Basically,
the memory in DOCA is managed by a struct called `doca_mmap` which is a
memory pool that holds the memory regions the user register into it. Also like 
RDMA, every DMA request(src, dst) should be on these registered memory
regions.

Here we wrap the struct `doca_mmap` into a struct `MemoryPool` as its
semantics are more like a memory pool rather than a memory mapping.

Like a remote Memory Region, in DOCA, the user need to fetch the metadata and
some extra information to construct a remote MemoryPool so the user can visit
the remote Memory(either local or remote). Hence, DOCA gives functions
called `export` and `create_from_export`. The first one export the extra
information about a MemoryPool and store it in memory. You can save it into
a file and transfer it to another side. After that, the user can load the file
into memory and create a remote MemoryPool using `create_from_export`.
The helper function `save_config_info_into_files` and `save_config_info_into_buffers`
are also provided.

The whole process can be seen in the picture below.![](https://docs.nvidia.com/doca/sdk/doca-core-programming-guide/graphics/doca-mmap-diagram.png)

### Buffer
The user can't use raw pointer or raw buffer to do DMA requests.
Theey should register the memory into memory pool and use a special 
struct `doca_buf` to point to the memory. The buffer module is the wrapper of these structs to control the Buffers pointed to the memory pool.

DOCA has another `doca_buf_inv` struct, which holds all `doca_buf`  
like a buffer repository. First the user need to allocate a `doca_buf`
from a `doca_buf_inv` and then the user can use the buffer. Notice that
there are `addr` and `data` fields in `doca_buf`. **The `data` field and `data_len`
is actually where the data is read/write**, so the user should call function `set_data`
before delivering a DMA request.

The relationship between Buffer and Memory module can be seen as the picture below.
![](https://docs.nvidia.com/doca/sdk/doca-core-programming-guide/graphics/doca-memory-subsystem-diagram.png)