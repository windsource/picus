VENDOR_DIR ?= vendor

CONFIG = .cargo/config.toml

TARGET_AMD64 ?= x86_64-unknown-linux-musl
TARGET_ARM64 ?= aarch64-unknown-linux-musl

# Determine the host architecture
ARCH := $(shell uname -m)

# Translate the output of uname -m to either amd64 or arm64
ifeq ($(ARCH),x86_64)
  ARCH := amd64
  DEFAULT_TARGET := $(TARGET_AMD64)
else ifeq ($(ARCH),amd64)
  ARCH := amd64
  DEFAULT_TARGET := $(TARGET_AMD64)
else ifeq ($(ARCH),aarch64)
  ARCH := arm64
  DEFAULT_TARGET := $(TARGET_ARM64)
else ifeq ($(ARCH),arm64)
  ARCH := arm64
  DEFAULT_TARGET := $(TARGET_ARM64)
else
  $(error Unsupported architecture: $(ARCH))
endif

# In CI builds we cannot write to the home directory
ifdef CI
  CARGO_HOME := $(shell pwd)/.cargo
  export CARGO_HOME
endif

ifdef CI_COMMIT_TAG
  # remove the leading 'v' from the tag
  VERSION = $(shell expr substr $(CI_COMMIT_TAG) 2 100)
  SOURCE_ARCHIVE = dist/picus-vendored-source-$(VERSION).tar.gz
  SOURCE_ARCHIVE_BASE = picus-$(VERSION)
else
  SOURCE_ARCHIVE = dist/picus-vendored-source.tar.gz
  SOURCE_ARCHIVE_BASE = picus
endif

all: test build-amd64 build-arm64 checksum

test:
	cargo test --target $(DEFAULT_TARGET)

build:
	cargo build --target $(DEFAULT_TARGET)

# Vendor sources and create source archive
vendor: Cargo.toml dist
	cargo vendor $(VENDOR_DIR)
	@if ! grep vendored-sources $(CONFIG); then \
	  echo '\n[source.crates-io]\nreplace-with = "vendored-sources"\n\n[source.vendored-sources]\ndirectory = "$(VENDOR_DIR)"' >> $(CONFIG); \
	fi
	# Note: The order is important in the next line. --exclude only affects items mentioned after it.
	#       So we can include .cargo/config.toml while excluding the rest of the folder.
	tar -czf $(SOURCE_ARCHIVE) --transform 's,^,$(SOURCE_ARCHIVE_BASE)/,' .cargo/config.toml --exclude=.cargo --exclude=target --exclude=dist --exclude=.git .

build-amd64: dist
	cargo build --release --target $(TARGET_AMD64)
	cp target/$(TARGET_AMD64)/release/picus dist/picus-linux-amd64

build-arm64: dist
	cargo build --release --target $(TARGET_ARM64)
	cp target/$(TARGET_ARM64)/release/picus dist/picus-linux-arm64

checksum: dist
	cd dist; sha256sum * > sha256sum.txt

dist:
	mkdir -p dist

check-licenses:
	cargo install cargo-deny
	cargo deny check licenses

clean:
	rm -rf target
	rm -rf $(VENDOR_DIR)
	# Revert changes for vendored sources
	git checkout -- $(CONFIG)
	rm -rf dist
