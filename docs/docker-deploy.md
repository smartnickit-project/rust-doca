# Deploy `rust-doca` with Docker

If the user encounter difficulties installing DOCA on the machine or would like to save time and effort, deploying rust-doca with Docker is a good option. [Docker](https://docs.docker.com/engine/install/) should be installed on the machine before following the instructions below.

`rust-doca` offers two methods for deploying with Docker.

## Build a Docker image with Rust installed inside

This method involves building a Docker image based on the native DOCA container image provided by Nvidia. `rust-doca` adds the Rust installation part so the user can start the container, compile, and run the entire project directly. One disadvantage of this method is that once the user delete the container, the newly started container needs to re-download all the crates. Therefore, it is important to use the same container and avoid deleting it.

`rust-doca` provides the [Dockerfile](../tools/Dockerfile) and [Makefile](../Makefile), making it easy to launch the container.

```Bash
# Build the `rust-doca` container image, `rust-doca:latest`
$ make build

# Launch the `rust-doca` container, which runs in the background.
$ make run

# Open a terminal inside the container to compile and run the project.
# You can build/run our repo inside this terminal
$ make open

# Delete the `rust-doca` container. Please be cautious!
$ make clean
```

If opening a terminal every time is inconvenient for the user, there is an alternative method provided by `rust-doca`. The script [`run_container.sh`](../tools/run_container.sh) launches a container and executes the command given by the user. Once the command completes, the container is automatically deleted. However, one downside of this approach is that the user may experience some delay as `cargo` updates the index.

## Run the Nvidia DOCA container with the Rust installed on the machine

This method does not require the use of the same container. The container will use the Rust installation on your machine instead of installing a new one inside the container. Simply run the command `make run_local` on the root directory, and a terminal will open inside the container for the user to compile and run the project.
