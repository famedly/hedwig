# Matrix Hedwig
This is a dead simple Push Gateway for a [Matrix.org](https://matrix.org) application. It implements the [Matrix Push Notification API r0.1.1](https://matrix.org/docs/spec/push_gateway/r0.1.1) and supports [Firebase Cloud Messaging](https://firebase.google.com/docs/cloud-messaging/) only.

## Features:
- Implements the `POST /_matrix/push/v1/notify` endpoint
- Forwards notifications from the format `event_id_only`
- Returns invalid push keys in the `rejected` response field
- Health status endpoint at `GET /health`
- Version endpoint at `GET /version`
- Prometheus metrics at `GET /metrics`

## Planned:
- Better logging

# Get started
1. Download the latest build from the CI: [amd64](https://gitlab.com/famedly/services/famedly-push-gateway-ng/-/jobs/artifacts/main/browse?job=cargo-build-amd64), [armv7](https://gitlab.com/famedly/services/famedly-push-gateway-ng/-/jobs/artifacts/main/browse?job=cargo-build-armv7), [aarch64](https://gitlab.com/famedly/services/famedly-push-gateway-ng/-/jobs/artifacts/main/browse?job=cargo-build-aarch64)

2. Add your Firebase Admin Key to the `config.yaml` file

3. Run the binary
```
./matrix-hedwig
```

## Data Messages and Notification Messages

In FCM there are two kind of messages: Data Messages and Notification Messages. Read more about it here: https://firebase.google.com/docs/cloud-messaging/concept-options

By default Hedwig is sending Notification Messages which payload you can configure in the `config.yaml`. To send Data Messages, just append `.data_message` to the APP ID.

## Proxy

You should configure a proxy with a working SSL connection to the gateway.

### Apache2 example

```
<Location "/_matrix/push/v1/">
  ProxyPass "http://localhost:7025/_matrix/push/v1/"
  SetEnv force-proxy-request-1.0 1
  SetEnv proxy-nokeepalive 1
</Location>
```

And optional:

```
<Location "/_matrix/push/health">
  ProxyPass "http://localhost:7025/health"
  SetEnv force-proxy-request-1.0 1
  SetEnv proxy-nokeepalive 1
</Location>

<Location "/_matrix/push/version">
  ProxyPass "http://localhost:7025/version"
  SetEnv force-proxy-request-1.0 1
  SetEnv proxy-nokeepalive 1
</Location>
```

## Docker

We provide a docker image with the compiled binary inside it. To use it, you need to map your
`config.yaml` into `/opt/hedwig/config.yaml` inside the container, and then you can route your
traffic to the configured listening port (default is `7022`).

Example usage with docker cli:

```
docker run --rm --name hedwig \
    -v ./config.toml:/opt/hedwig/config.toml \
    -p 127.0.0.1:7022:7022 \
    registry.gitlab.com/famedly/services/hedwig:latest
```

# How to build for your platform

1. [Install Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

2. Build the binary:
```
cargo build --release
```
