# Do not exit when variable is unbound (standard is "sh -cu")
set shell := ["sh", "-c"] 

vendor_dir := "vendor"

config := ".cargo/config.toml"

target_amd64 := "x86_64-unknown-linux-musl"
target_arm64 := "aarch64-unknown-linux-musl"

# We need to use musl-gcc to build the binary
# See https://github.com/aws/aws-lc-rs/issues/736
cc_amd64 := "/usr/bin/x86_64-linux-musl-gcc"
cc_arm64 := "/usr/bin/aarch64-linux-musl-gcc"

default_target := if arch() == "x86_64" { 
    target_amd64
} else if arch() == "aarch64" { 
    target_arm64
} else {
    error("Unknwon host architecture")
}

default_cc := if arch() == "x86_64" { 
    cc_amd64
} else if arch() == "aarch64" { 
    cc_arm64
} else {
    error("Unknwon host architecture")
}

# In CI builds we cannot write to the home directory
export CARGO_HOME := if `echo $CI` != "" {
    justfile_directory() / ".cargo"
} else {
    env("CARGO_HOME", env("HOME") / ".cargo") 
}

all: check-licenses test build-amd64 build-arm64 checksum

test:
    CC={{default_cc}} cargo test --target {{default_target}}

# Debug build for host arch
build:
    CC={{default_cc}} cargo build --target {{default_target}}

build-amd64: _dist
    CC={{cc_amd64}} cargo build --release --target {{target_amd64}}
    cp target/{{target_amd64}}/release/picus dist/picus-linux-amd64

build-arm64: _dist
    CC={{cc_arm64}} cargo build --release --target {{target_arm64}}
    cp target/{{target_arm64}}/release/picus dist/picus-linux-arm64

checksum: _dist
    cd dist; sha256sum * > sha256sum.txt

_dist:
    mkdir -p dist

check-licenses:
    cargo install cargo-deny@0.19.0
    cargo deny check licenses

# Vendor sources and create source archive
vendor: _dist
    #!/bin/sh -e
    cargo vendor {{vendor_dir}}
    if ! grep vendored-sources {{config}}; then
      echo '\n[source.crates-io]\nreplace-with = "vendored-sources"\n\n[source.vendored-sources]\ndirectory = "{{vendor_dir}}"' >> {{config}};
    fi
    if [ -n "$CI_COMMIT_TAG" ]; then
        # remove the leading 'v' from the tag
        VERSION=$(expr substr "$CI_COMMIT_TAG" 2 100)
        SOURCE_ARCHIVE=dist/picus-vendored-source-${VERSION}.tar.gz
        SOURCE_ARCHIVE_BASE=picus-${VERSION}
    else
        SOURCE_ARCHIVE=dist/picus-vendored-source.tar.gz
        SOURCE_ARCHIVE_BASE=picus
    fi
    # Note: The order is important in the next line. --exclude only affects
    #       items mentioned after it. So we can include .cargo/config.toml
    #       while excluding the rest of the folder.
    tar -czf ${SOURCE_ARCHIVE} --transform "s,^,${SOURCE_ARCHIVE_BASE}/," .cargo/config.toml --exclude=.cargo --exclude=target --exclude=dist --exclude=.git .

clean:
    rm -rf target
    rm -rf {{vendor_dir}}
    # Revert changes for vendored sources
    git checkout -- {{config}}
    rm -rf dist
