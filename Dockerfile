# -*- mode: dockerfile -*-

# You can override this `--build-arg BASE_IMAGE=...` to use different
# version of Rust or OpenSSL.
ARG BASE_IMAGE=ekidd/rust-musl-builder:stable

# Our first FROM statement declares the build environment.
FROM ${BASE_IMAGE} AS builder

# Add our source code.
ADD --chown=rust:rust . ./

# Build our application.
RUN cargo build --release

# Now build the _real_ Docker container, copying in the release binary
FROM alpine:latest
RUN apk --no-cache add ca-certificates
COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/gestetner \
    /usr/local/bin/
ENV RUST_LOG="gestetner=info"
ENV GESTETNER_HOST="http://localhost"
ENV PORT="5000"
CMD ["/bin/sh", "-c", "/usr/local/bin/gestetner -l \"[::]:9999\" -p /var/run/gestetner -u $GESTETNER_HOST -w \"[::]:$PORT\""]

