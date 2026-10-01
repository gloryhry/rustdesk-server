# syntax=docker/dockerfile:1
FROM --platform=$BUILDPLATFORM node:24.9.0-bookworm-slim AS web-build
WORKDIR /source/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/index.html web/vite.config.js ./
COPY web/src ./src
RUN npm run build

# Fetch locked sources on the build host: 32-bit libgit2 cannot read large overlay inode numbers.
FROM --platform=$BUILDPLATFORM rust:1.98.0-bookworm AS dependency-source
WORKDIR /source
COPY Cargo.toml Cargo.lock build.rs ./
COPY libs/hbb_common ./libs/hbb_common
COPY vendor ./vendor
COPY src ./src
RUN --mount=type=cache,id=rustdesk-source-registry,target=/usr/local/cargo/registry --mount=type=cache,id=rustdesk-source-git,target=/usr/local/cargo/git \
    cargo vendor --locked /source/vendor-dependencies > /source/vendor-config.toml

FROM rust:1.98.0-bookworm AS rust-build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev libclang-dev cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /source
COPY Cargo.toml Cargo.lock build.rs ./
COPY libs/hbb_common ./libs/hbb_common
COPY vendor ./vendor
COPY src ./src
COPY --from=dependency-source /source/vendor-dependencies ./vendor-dependencies
COPY --from=dependency-source /source/vendor-config.toml ./.cargo/config.toml
ARG RUST_PROFILE=release
ARG CARGO_BUILD_JOBS=2
ARG TARGETPLATFORM
RUN --mount=type=cache,id=rustdesk-target-${TARGETPLATFORM},target=/source/target,sharing=locked \
    set -eu; touch build.rs; cargo build --locked --offline --profile "$RUST_PROFILE" --bin hbbs --bin hbbr --bin rustdesk-api --bin rustdesk-utils; \
    mkdir /binaries; case "$RUST_PROFILE" in dev) profile_dir=debug ;; *) profile_dir="$RUST_PROFILE" ;; esac; \
    cp "/source/target/$profile_dir/hbbs" "/source/target/$profile_dir/hbbr" "/source/target/$profile_dir/rustdesk-api" "/source/target/$profile_dir/rustdesk-utils" /binaries/

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libgcc-s1 libssl3 libstdc++6 curl util-linux netcat-openbsd && rm -rf /var/lib/apt/lists/* && \
    mkdir /data && chown 10001:10001 /data
COPY --from=rust-build /binaries/ /usr/bin/
COPY --from=web-build /source/web/dist/ /usr/share/rustdesk-api-web/
COPY --chmod=755 docker/entrypoint.sh /usr/local/bin/rustdesk-entrypoint
ENV DB_URL=/data/db_v2.sqlite3 RUSTDESK_KEY_FILE=/data/id_ed25519.pub API_WEB_ROOT=/usr/share/rustdesk-api-web API_ENABLED=0
WORKDIR /data
USER 10001:10001
EXPOSE 21114 21115 21116 21116/udp 21117 21118 21119
ENTRYPOINT ["/usr/local/bin/rustdesk-entrypoint"]
CMD ["rustdesk-api"]

FROM runtime AS supervisor
USER root
ARG S6_OVERLAY_VERSION=3.2.0.0
ARG S6_ARCH=x86_64
RUN apt-get update && apt-get install -y --no-install-recommends xz-utils && rm -rf /var/lib/apt/lists/*
ADD https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-noarch.tar.xz /tmp/s6-overlay-noarch.tar.xz
ADD https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-${S6_ARCH}.tar.xz /tmp/s6-overlay-arch.tar.xz
RUN tar -C / -Jxpf /tmp/s6-overlay-noarch.tar.xz && tar -C / -Jxpf /tmp/s6-overlay-arch.tar.xz && rm /tmp/s6-overlay*.tar.xz
COPY docker/rootfs/ /
ENV API_ENABLED=1 S6_BEHAVIOUR_IF_STAGE2_FAILS=2
HEALTHCHECK --interval=10s --timeout=5s CMD /usr/bin/healthcheck.sh
ENTRYPOINT ["/init"]
CMD []
