FROM registry.famedly.net/docker-oss/rust-container:nightly AS builder
ARG CARGO_NET_GIT_FETCH_WITH_CLI=true
ARG CARGO_BUILD_RUSTFLAGS
ARG CI_SSH_PRIVATE_KEY

# Add CI key for git dependencies in Cargo.toml. This is only done in the builder stage, so the key
# is not available in the final container.
RUN mkdir -p ~/.ssh
RUN echo "${CI_SSH_PRIVATE_KEY}" > ~/.ssh/id_ed25519
RUN chmod 600 ~/.ssh/id_ed25519
RUN echo "Host *\n\tStrictHostKeyChecking no\n\n" > ~/.ssh/config

COPY . /app
WORKDIR /app
RUN cargo auditable build --release

# distroless cc: glibc, libgcc, OpenSSL (with its zlib and zstd), CA certificates and tzdata; no shell, no
# package manager. Debian 13 like the builder: the binary needs the builder's glibc or newer.
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cb87acc574abe719b6e5c547384defa2f7b1f24af9605d3f618cf852495d8be2 AS hedwig

WORKDIR /opt/matrix-hedwig
COPY --from=builder /app/target/release/matrix-hedwig /usr/local/bin/matrix-hedwig
# Fails the build if the binary cannot run on this base (2.5.1 shipped an image that could not).
RUN ["/usr/local/bin/matrix-hedwig", "--version"]
ENV TZ=Etc/UTC
# This port number should match the number set in `config.sample.yaml`
EXPOSE 7022/tcp
# Probes server.port of the same config.yaml; distroless has no curl.
HEALTHCHECK --interval=3s --timeout=3s --retries=2 --start-period=5s CMD ["/usr/local/bin/matrix-hedwig", "health"]
CMD ["/usr/local/bin/matrix-hedwig"]
