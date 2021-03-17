FROM docker.io/alpine:edge as builder
RUN apk add --no-cache \
      cargo \
      build-base \
      openssl-dev \
      git
COPY . /app
WORKDIR /app
RUN cargo build --release

FROM docker.io/alpine:edge
RUN apk add --no-cache \
      libgcc \
      libssl1.1 \
      curl \
  && mkdir -p /opt/hedwig
WORKDIR /opt/hedwig
COPY --from=builder /app/target/release/matrix-hedwig /usr/local/bin/matrix-hedwig
CMD ["/usr/local/bin/matrix-hedwig"]
