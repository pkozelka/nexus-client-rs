all:

NEXUS_AUTH=$(shell sed -n '/^machine oss.sonatype.org/{s:^[^#].*login ::;s# password #:#;p;}' ~/.netrc)
VERSION=0.1.0
DIST=target/nexus-client-rs-0.1.0

release:
	rm -rf $(DIST)
	mkdir -p $(DIST)
	cp -v target/release/nexus $(DIST)/
	tar zcf $(DIST).tgz -C target nexus-client-rs-0.1.0

target/release/nexus: Cargo.toml $(find src -type f)
	cargo build --release

.PHONY: docker-build docker-run

install: target/release/nexus
	cargo install --path .

docker-build: Dockerfile target/release/nexus
	rm -rf target/docker
	mkdir -p target/docker
	cp -t target/docker Dockerfile target/release/nexus
	cd target/docker \
	&& docker build --tag nexus-client-rs .

DO=sh
in-docker-run: docker-build
	docker run -it --rm --network host \
	  -w /code \
	  -e NEXUS_AUTH=$(NEXUS_AUTH) \
	  -e RUST_LOG=info \
	  -v $(PWD):/code \
	  -v $(HOME)/.netrc:/root/.netrc \
	  nexus-client-rs $(DO)

nexus-staging-repos:
	make in-docker-run DO="nexus staging repos"

install-just:
	cargo install --git https://github.com/casey/just just
