# Build an image that uses debian slim as runtime
#
# I also tried to run it on Alpine as this would create a smaller image size
# but that did not work. Build worked fine but when I started the picus app it
# crashed when a REST API was called. The only output was
# 'Segmentation fault (core dumped)' but there was no core dump. This seems to
# be in line with 
# https://www.reddit.com/r/rust/comments/sq53vx/alpine_fails_to_run_my_app_what_steps_should_i/
# So finally I gave up and used debian slim as runtime.


FROM rust:1.63.0-slim-bullseye as builder

RUN apt update && apt install -y libssl-dev pkg-config

WORKDIR /usr/src

# Create blank project
RUN USER=root cargo new picus

# We want dependencies cached, so copy those first.
COPY Cargo.toml Cargo.lock /usr/src/picus/

WORKDIR /usr/src/picus

# This is a dummy build to get the dependencies cached.
RUN cargo build --release

# Now copy in the rest of the sources
COPY src /usr/src/picus/src/

## Touch main.rs to prevent cached release build
RUN touch /usr/src/picus/src/main.rs

RUN cargo test

# This is the actual application build.
RUN cargo build --release

### Runtime
FROM debian:11.5-slim AS runtime

RUN apt update && apt install -y ca-certificates

RUN groupadd -g 999 appuser && \
    useradd -r -u 999 -g appuser appuser
USER appuser

COPY --from=builder /usr/src/picus/target/release/picus /usr/local/bin

CMD ["/usr/local/bin/picus"]
