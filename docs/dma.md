## DOCA DMA Intro
doca dma is a data-path library based on NVIDIA DPU
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
and open it for later usage. `rust-doca` give the users a useful 
function `open_device_with_pci` so that they can open a doca device 
directly.

Notice that the user can find DOCA devices by command `lspci | grep Mellanox`
```bash
17:00.0 Ethernet controller: Mellanox Technologies MT42822 BlueField-2 integrated ConnectX-6 Dx network controller (rev 01)
17:00.1 Ethernet controller: Mellanox Technologies MT42822 BlueField-2 integrated ConnectX-6 Dx network controller (rev 01)
```

Here, due to that the link layer of the DPU is `Ethernet`, the output is `Ethernet controller`. If the link layer is IB, the output should be `Infiniband controller`.

### Memory
Memory is an important module in DOCA especially for DOCA DMA. Basically,
the memory in DOCA is managed by a struct called `doca_mmap`, which is a
memory pool that holds the memory regions the user registered into it. Like 
RDMA, every DMA request's source and destination should be on these registered memory
regions.

Like a remote Memory Region in RDMA, in DOCA, the user need the metadata 
to construct a remote Memory map so the user can visit
the remote Memory(either on local or remote side). Hence, `doca_mmap` gives the functions: `export` and `create_from_export`. `export` exports the metadata
information of a Memory map and store it in the memory. The user can choose their
own way to transfer the information to the other side(e.g. the user can save it into
a file and copy the file to another side. After that, the user can load the file
into memory and create a remote Memory map using `create_from_export`.)
`rust-doca` provides the helper function `save_config_info_into_files` and `save_config_info_into_buffers` to achieve this.

The whole process can be seen in the picture below.![](https://docs.nvidia.com/doca/sdk/doca-core-programming-guide/graphics/doca-mmap-diagram.png)

### Buffer
In DOCA, the user can't use raw pointer or raw buffer to deliver DMA requests.
User should register the memory into memory map and use a 
struct called `doca_buf` to point to the memory.

DOCA also has buffer related struct called `doca_buf_inv`, which holds all 
allocated `doca_buf` like a buffer repository. User needs to allocate a `doca_buf`
from a `doca_buf_inv` before user can use the buffer. Notice that
there are `addr` and `data` fields in `doca_buf`. **The `data` field and `data_len`
is actually where the data is read/write**, so the user should call function `set_data`
before delivering a DMA request.

The relationship between Buffer and Memory module can be seen as the picture below.
![](https://docs.nvidia.com/doca/sdk/doca-core-programming-guide/graphics/doca-memory-subsystem-diagram.png)