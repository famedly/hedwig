FROM ghcr.io/famedly/rust-container:nightly as builder
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

FROM debian:bullseye-slim

RUN apt-get update -qq -o Acquire::Languages=none && \
    env DEBIAN_FRONTEND=noninteractive apt-get install \
    -yqq \
# install...
        ca-certificates \
        tzdata \
        curl && \
# clean up...
    rm -rf /var/lib/apt/lists/* && \
# create working directory
    mkdir -p /opt/matrix-hedwig && \
# ensure the UTC timezone is set
    ln -fs /usr/share/zoneinfo/Etc/UTC /etc/localtime

WORKDIR /opt/matrix-hedwig
COPY --from=builder /app/target/release/matrix-hedwig /usr/local/bin/matrix-hedwig
CMD ["/usr/local/bin/matrix-hedwig"]
ENV TZ=Etc/UTC
# This port number should match the number set in `config.sample.yaml`
ARG service_port_number=7022
EXPOSE ${service_port_number}/tcp
ENV SERVICE_PORT=${service_port_number}
HEALTHCHECK --interval=3s --timeout=3s --retries=2 --start-period=5s \
 CMD curl -fSs http://localhost:$SERVICE_PORT/health || exit 1