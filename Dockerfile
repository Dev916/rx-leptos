# syntax=docker/dockerfile:1
# Builds the SSR example (examples/leptos-ssr) for Cloudflare Containers.
# The build stage runs on the host architecture and cross-compiles the
# server for linux/amd64; the wasm client is architecture-independent.

FROM --platform=$BUILDPLATFORM rust:1.93-bookworm AS build
RUN apt-get update \
 && apt-get install -y --no-install-recommends gcc-x86-64-linux-gnu libc6-dev-amd64-cross ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown x86_64-unknown-linux-gnu \
 && curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CARGO_NET_GIT_FETCH_WITH_CLI=true
WORKDIR /src
COPY . .
WORKDIR /src/examples/leptos-ssr
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --no-default-features --features ssr --target x86_64-unknown-linux-gnu \
 && cp target/x86_64-unknown-linux-gnu/release/leptos-ssr-example /leptos-ssr-example
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    wasm-pack build --release --target web --no-typescript --no-pack \
      --out-dir /site/pkg --out-name leptos-ssr-example \
      -- --no-default-features --features hydrate \
 && cp /site/pkg/leptos-ssr-example_bg.wasm /site/pkg/leptos-ssr-example.wasm \
 && cp -r public/. /site/

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /leptos-ssr-example /app/leptos-ssr-example
COPY --from=build /site /app/site
ENV LEPTOS_OUTPUT_NAME=leptos-ssr-example \
    LEPTOS_SITE_ROOT=/app/site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:3000
EXPOSE 3000
CMD ["/app/leptos-ssr-example"]
