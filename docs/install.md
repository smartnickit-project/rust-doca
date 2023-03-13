# DOCA Installation on host & DPU

## 0. Pre-requests

Related software can be downloaded in the following links. 

1. [DOCA Intro](https://developer.nvidia.com/networking/doca)
2. [DOCA SDK](https://docs.nvidia.com/doca/sdk/installation-guide-for-linux/index.html#manual-bluefield-image-installation)

Link 1 is **strongly** recommended as it doesn't required you to login in with Nvidia accout.

## 1. Install DOCA on the host 

Host is typically an x86_64 architecture, and we use Ubuntu18.04 here to show how to install DOCA as reference.

### Install with apt-get

Usually, DOCA can be easily installed with the following commands:

```bash
wget https://www.mellanox.com/downloads/DOCA/DOCA_v1.5.1/doca-host-repo-ubuntu1804_1.5.1-0.1.8.1.5.1007.1.5.8.1.1.2.1_amd64.deb
sudo dpkg -i doca-host-repo-ubuntu1804_1.5.1-0.1.8.1.5.1007.1.5.8.1.1.2.1_amd64.deb
sudo apt-get update
sudo apt install doca-runtime
sudo apt install doca-tools
sudo apt install doca-sdk
```

These commands can be found in reference 1's `BlueField Drivers` section, you can switch to your own OS to get the correct commands.

However, if you found `fail to overwrite` error when executing `dpkg -i` command, use the follow command to replace it:

```bash
sudo dpkg --force-overwrite -i doca-host-repo-ubuntu1804_1.5.1-0.1.8.1.5.1007.1.5.8.1.1.2.1_amd64.deb
```

If you encouter some dependency error during installing, just fix them with:

```bash
sudo apt-get install <pkg>=<version>
```

If all apt-get commands exits normally, DOCA installation is finished.

### Install with BFB

A BFB is a packaged Linux OS with full support for DOCA, so it is recommended to install with this way when configuring new bare metals.

BFB can be download either from https://developer.nvidia.com/networking/doca or https://docs.nvidia.com/doca/sdk/installation-guide-for-linux/index.html#manual-bluefield-image-installation . After the downloads, use the following command to install BFB OS:

```bash
sudo bfb-install --bfb <image_path.bfb>
```

The remaining steps is nothing but following bfb-install's guidance.

## 2. Install the DOCA on the DPU

We use Ubuntu 20.04 of aarch64 to show to install DOCA on DPU.

## Install with apt-get

Download DOCA repo for DPU(e.g DOCA 1.5.1 LTS):

```bash
wget https://content.mellanox.com/DOCA/DOCA_v1.5.1/doca-dpu-repo-ubuntu2004-local_1.5.1007-1.5.8.1.1.2.0.bf.3.9.3.12383.4.2211-LTS.signed_arm64.deb
sudo dpkg -i doca-dpu-repo-ubuntu2004-local_1.5.1007-1.5.8.1.1.2.0.bf.3.9.3.12383.4.2211-LTS.signed_arm64.deb
sudo apt-get update
sudo apt install doca-runtime
sudo apt install doca-tools
sudo apt install doca-sdk
```

Again, if you encouter some dependency error during installing, just fix them with:

```bash
sudo apt-get install <pkg>=<version>
```

If all apt-get commands exits normally, DOCA installation is finished.

## Install with BFB

Download the aarch64 BFB, then run with:

```bash
sudo bfb-install --rshim <rshimN> --bfb <image_path.bfb>
```

Notive that `rshimN` is `rshim0` if you only have one DPU according to [DOCA SDK](https://docs.nvidia.com/doca/sdk/installation-guide-for-linux/index.html#option-1-no-pre-defined-password).

