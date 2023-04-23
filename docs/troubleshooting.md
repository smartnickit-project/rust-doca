# Troubleshooting

**Note: `rust-doca` currently only supports DOCA 1.5, as different versions of DOCA may have non-identical APIs.**

## Problems with Compilation

In most cases, problems with Compilation are caused by a failed installation or an incompatible version of DOCA.

### error[E0425]: cannot find function `doca_xxx` in crate `ffi`

This error is caused by a version of DOCA that is too low. You can find the version of DOCA currently installed on the machine by checking the suffix of the `.so` files in the path `/opt/mellanox/doca/lib/${ARCH}-linux-gnu`, such as `libdoca_common.so.1.5.1007`.

**Solution**: Try to install the DOCA 1.5 using the [tutorial](install.md), or use Docker to build
and run the application following the [Docker deployment](docker-deploy.md).

### fatal error: 'doca_xxx.h' file not found

This error occurs when `doca-runtime` is not yet installed, and the `bindgen` couldn't find the corresponding headers in `/opt/mellanox/doca/include`.

**Solution**: Run `sudo apt install doca-runtime`. If this fails, try updating your system(such as kernel version) or use Docker as described above.

## Problems in Running

### DMA work queue context unable to create QP. err=DOCA_ERROR_NO_MEMORY

Why this error happen remains unknown. This error may occur when the user uses multiple threads to initialize DOCA structures(e.g. when initializing all parts in benchmark)

**Solution**: Try adding a sleep command for a few milliseconds between each thread's creation.

### CQ received for failed job: status=2, vendor error=104

Why this error happen remains unknown. It might due to the wrong setup for source buffer and destination buffer in DOCA. The vendor error means that the driver issued a WQE that was malformed in size.

There is no universal solution to this problem.

If you encounter any issues that are not covered in this guide, please don't hesitate to reach out to us at [yangfisher01@gmail.com](yangfisher01@gmail.com). We are always happy to help.


### Large region not permitted

In DOCA DMA bench, if you register a too large buffer to DOCA, you might end up with get an error code of `DOCA_ERROR_NOT_PERMITTED`. The reason is similar to `ibv_reg_mr` in RDMA, in that DOCA needs to lock the region you want to register, and the size of region you can lock is limited. 

You can set the limitation to unlimited by do the following, first update `limits.conf`:

```bash
vim /etc/security/limits.conf

# add 2 rows to the file
* soft memlock unlimited
* hard memlock unlimited
```

Then, set the limitation to unlimited:

```bash
ulimit -l unlimited
```