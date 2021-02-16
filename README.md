# Matrix Hedwig
This is a dead simple Push Gateway for a [Matrix.org](https://matrix.org) application. It implements the [Matrix Push Notification API r0.1.1](https://matrix.org/docs/spec/push_gateway/r0.1.1) and supports [Firebase Cloud Messaging](https://firebase.google.com/docs/cloud-messaging/) only.

## Features:
- Implements the `POST /_matrix/push/v1/notify` endpoint
- Forwards notifications from the format `event_id_only`
- Returns invalid push keys in the `rejected` response field

## Planned:
- Better logging
- Endpoint for stats

# Get started
1. Download the latest build from the CI: [amd64](https://gitlab.com/famedly/services/famedly-push-gateway-ng/-/jobs/artifacts/main/browse?job=cargo-build-amd64), [armv7](https://gitlab.com/famedly/services/famedly-push-gateway-ng/-/jobs/artifacts/main/browse?job=cargo-build-armv7)

2. Add your Firebase Admin Key to the `config.toml` file

3. Run the binary

# How to build for your platform

1. [Install Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

2. Build the binary:
```
cargo build --release
```

## Build Dependencies:
- openSSL

##### Fedora:
```
sudo dnf install openssl-devel
```

##### Debian/Ubuntu:
```
sudo apt install openssl-dev
```

# This project is part of the source code of Famedly

We think that software for healthcare should be open source, so we publish most 
parts of our source code at [gitlab.com/famedly](https://gitlab.com/famedly).

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of
conduct, and the process for submitting pull requests to us.

For licensing information of this project, have a look at the [LICENSE.md](LICENSE.mD)
file within the repository.

If you compile the open source software that we make available to develop your
own mobile, desktop or embeddable application, and cause that application to
connect to our servers for any purposes, you have to aggree to our Terms of
Service. In short, if you choose to connect to our servers, certain restrictions
apply as follows:  

* You agree not to change the way the open source software connects and
interacts with our servers
* You agree not to weaken any of the security features of the open source software
* You agree not to use the open source software to gather data
* You agree not to use our servers to store data for purposes other than
the intended and original functionality of the Software
* You acknowledge that you are solely responsible for any and all updates to
your software

No license is granted to the Famedly trademark and its associated logos, all of
which will continue to be owned exclusively by Famedly GmbH. Any use of the
Famedly trademark and/or its associated logos is expressly prohibited without
the express prior written consent of Famedly GmbH.

For more
information take a look at [Famedly.com](https://famedly.com) or contact
us by [info@famedly.com](mailto:info@famedly.com?subject=[GitLab]%20More%20Information%20)

---