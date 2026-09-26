# Official multi-architecture Rust image; execute natively on each Linux runner.
FROM rust:1.94-alpine3.23@sha256:77237dd363a0b127bb5ef532c2d64c0deb380b738e43a9c4bdac73398d6d0a08
RUN apk add --no-cache nodejs npm build-base && node -e "if (process.versions.node.split('.')[0] !== '24') process.exit(1)"
