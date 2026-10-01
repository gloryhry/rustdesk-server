# syntax=docker/dockerfile:1
FROM node:24.9.0-bookworm-slim AS web-build
WORKDIR /source/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/index.html web/vite.config.js ./
COPY web/src ./src
RUN npm run build

FROM rust:1.98.0-bookworm AS rust-build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev libclang-dev cmake && rm -rf /var/lib/apt/lists/*
WORKDIR /source
COPY Cargo.toml Cargo.lock build.rs ./
COPY libs/hbb_common ./libs/hbb_common
COPY vendor/users-compat ./vendor/users-compat
COPY src ./src
ARG RUST_PROFILE=release
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git --mount=type=cache,target=/source/target \
    set -eu; touch build.rs; cargo build --locked --profile "$RUST_PROFILE" --bin hbbs --bin hbbr --bin rustdesk-api --bin rustdesk-utils; \
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
