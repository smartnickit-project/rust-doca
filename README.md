# Rust-DOCA

Rust API wrapper for the NVIDIA `DOCA` SDK.

The NVIDIA `DOCA` SDK enables developers to rapidly create applications and services on
 top of NVIDIA® BlueField® data processing units (DPUs), leveraging industry-standard
  APIs. With DOCA, developers can deliver breakthrough networking, security, and
   storage performance by harnessing the power of NVIDIA's DPUs.

For more information on `DOCA` SDK, please refer to the [DOCA SDK Document](https://docs.nvidia.com/doca/sdk/index.html). The user can also find the original C definitions on the website.

A good place to start is to look at the programs in [`doca/examples/`](doca/examples/) (whose example is listed in the `README.md` at its folder), 
and the original (corresponding) C examples which can be found at `/opt/mellanox/doca/samples` if DOCA is installed on the machine.
To save user's time and effort, [deploying rust-doca with Docker](docs/docker-deploy.md) is a good option.

## Library dependency
The `rust-doca` crate is totally supported by `DOCA` SDK.  If the machine has DOCA SDK installed, the user can easily find it at the path `/opt/mellanox/doca`. If not, the user may need to install the SDK by following the
the [installation tutorial](
    docs/install.md
) to install it.
If you don't want to install docker (or have trouble installing it), 
please see the [deploying rust-doca with Docker](docs/docker-deploy.md).

To verify the installation is complete with the following:
```
cargo test 
```

## Documentation
If the user encounters any issues with this crate, please refer to [Troubleshooting Guide](docs/troubleshooting.md), [API Library](https://docs.nvidia.com/doca/sdk/doca-libraries-api/index.html), and
[Core Program Guide](https://docs.nvidia.com/doca/sdk/doca-core-programming-guide/index.html) for help.

## Roadmap
- [x] Support DOCA DMA
- [ ] Support DOCA Comm Channel
- [ ] Support other DOCA usage 