# Make sure that the docker image `rust-doca` has been built first!
docker run -v $(dirname $PWD):/rust-doca --privileged --workdir=/rust-doca --rm -e container=docker rust-doca $@
