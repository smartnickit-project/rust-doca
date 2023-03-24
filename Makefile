
build:
	docker build -t rust-doca tools/.

run:
	docker run -v $(PWD):/rust-doca --privileged --name doca-builder --workdir=/rust-doca -itd -e container=docker rust-doca

run_local:
	docker run --rm -v $(PWD):/rust-doca -v ~/.cargo:/root/.cargo -v ~/.rustup:/root/.rustup  --privileged --name doca-builder --workdir=/rust-doca -it -e container=docker -e PATH="/root/.cargo/bin:${PATH}"  nvcr.io/nvidia/doca/doca:1.5.1-devel

clean:
	docker rm -f doca-builder

open:
	docker exec -it doca-builder bash
