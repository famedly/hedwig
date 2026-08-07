# Changelog

All notable changes to this project will be documented in this file.

## [2.4.0] - 2026-08-07

### 🚀 Features

- validate VoIP devices against a separate app_id prefix
- add VoIP push support for PushKit / CallKit

### ⚙️ Miscellaneous Tasks

- update GitHub Action pins

## [2.3.0] - 2026-01-21

### 🚀 Features

- redirect `/` endpoint to `/version`

### 🐛 Bug Fixes

- conditionally include the keys `category`, `mutable_content` and `content-available` in aps payload

### ⚙️ Miscellaneous Tasks

- clarify how to disable APNS in the config
- remove unused k8s configuration
- update rust dependencies
- allow any otel sdk version in prometheus output for the tests to pass

## [2.2.0] - 2026-01-09

### 🚨 BREAKING changes: Configuration Schema changes

Hedwig will fail to start without adjustment to your configuration!

Most existing configuration settings have been renamed(if you had a setting that started with `fcm_`, you will have to make a change). Specific settings for Android(FCM) and iOS(FCM or APNS) have been added. New optional telemetry settings have been added.

Previously, Hedwig configuration was fairly simple. The new changes are extensive, and it is recommended you start from a fresh configuration file based on the provided sample config yaml. Copying existing settings to this new file should be fairly straight forward.

The FCM (google) auth has also changed, you either need to set the config var `fcm_credentials_file_path` pointing to your existing credentials file, or use one of the methods listed [here](https://github.com/djc/gcp_auth/blob/5a1e48db47784c9afdbad38a33907cb2e98bbfdd/README.md).

Support for direct APNS is optional, config related to it isn't mandatory.

### 🚀 Features

- add support for most fcm and apns notification keys and headers
- add direct APNS support
- use rust-metrics instead of axum-opentelemetry-middleware
- configurable apns-push-type header
- add CatchPanic middleware
- Remove kustomize config and update Dockerfile to new openshift standards

### ⚙️ Miscellaneous Tasks

- update crates to mitigate possible vulnerabilities

## [2.0.0] - 2024-06-21

### 🚀 Features

- Build release builds with cargo-auditable
- [**breaking**] Remove jitter functionality
    - This entirely removes all jitter functionality from Hedwig, including the relevant metrics.

### 🐛 Bug Fixes

- Update docker pipeline
- Use correct builder image
- Re-add device and notification metrics

### 🚜 Refactor

- Define lifetime of static str

### 📚 Documentation

- Set health check and time zone

### ⚙️ Miscellaneous Tasks

- Fix clippy lints
- Remove vetting remains
- Migrate to github
- Fix permissions for coverage in dependabot PRs
- Add custom prefix to dependabot commits
- Update to new reusable workflow
- Move lints to Cargo.toml
- Add hosted rustdoc documentation
- Remove dependabot configuration

### Bump

- Bump serde_json from 1.0.104 to 1.0.107 (#125)
- Bump reqwest from 0.11.18 to 0.11.22 (#122)
- Bump async-trait from 0.1.72 to 0.1.73 (#124)

## [1.5.5] - 2023-08-10

### 🐛 Bug Fixes

- Use https git dependency to prevent ssh fingerprint errors when building locally
- Update gcp_auth to avoid exposing secrets in logs

### ⚙️ Miscellaneous Tasks

- Correct container URL in Dockerfile
- Update codeowners to reflect reality
- Update Axum to 0.6.1
- *(bot)* Update files from template
- Update codeowners
- Add docker-compose
- Add github action
- Update CODEOWNERS to github structure
- Update git dependencies
- Add docker build job
- Delete gitlab CI file
- Remove description key
- Remove registry ref from docker build

### Bump

- Release v1.5.5

## [1.5.3] - 2022-10-05

### 🚀 Features

- Improve test coverage

### ⚙️ Miscellaneous Tasks

- Audit dependencies with cargo-vet
- *(audit)* Cfg-if, firebae-cm and tinyvec_macros
- Add curl to dockerfile
- Bump to version 1.5.3

## [1.5.2] - 2022-09-11

### 🚀 Features

- Add config option to log into file instead of stdout

### 🚜 Refactor

- Get rid of expects in runtime code

### ⚙️ Miscellaneous Tasks

- Update axum
- Bump to version 1.5.2

## [1.5.1] - 2022-08-12

### 🐛 Bug Fixes

- Don't send notification on iOS legacy when only updating badge

### ⚙️ Miscellaneous Tasks

- Update version number

## [1.5.0] - 2022-08-01

### 🚀 Features

- Better badge behaviour in background-handled iOS notifications

### 🐛 Bug Fixes

- Add retry loop for pushing to fcm

## [1.4.0] - 2022-07-14

### 🚀 Features

- Rewrite in axum + fcmv1

### 🐛 Bug Fixes

- Make dendrite pushes work

## [1.3.0] - 2021-12-17

### 🚀 Features

- Add coverage
- Code coverage
- Add encrypted push fields

### 🚜 Refactor

- Idiomatic refactor, documentation

### ⚙️ Miscellaneous Tasks

- Fallback for git build metadata
- Add/update pre-commit
- *(bot)* Update files from template
- Bump to 1.3.0

### Chore

- Add badges to readme

## [1.2.0] - 2021-06-22

### 🐛 Bug Fixes

- Send clearing badge for non data messages
- CI project path

### ⚙️ Miscellaneous Tasks

- Version bump to 1.2.0
- Cargo fmt

## [1.1.0] - 2021-05-31

### 🚀 Features

- Send non-collapsible messages

### 🐛 Bug Fixes

- Remove collapse key from settings
- *(ci)* Use updated and corrected template

### ⚙️ Miscellaneous Tasks

- Version bump to 1.1.0

## [1.0.0] - 2021-04-30

### 🚀 Features

- Better unread notification handling
- *(ci)* Migrate to templates in pipeline

### 🐛 Bug Fixes

- *(ci)* Include new fix in upstream template
- Ci fix attempt
- Ci pipeline fix attempt

### 🚜 Refactor

- Configuration standardized

### 📚 Documentation

- Explain notification types

### ⚙️ Miscellaneous Tasks

- Version bump to 1.0.0

### Chore

- Update Readme

### Fix

- Links for documentation

## [0.1.0] - 2021-03-29

### 🚀 Features

- Add logging
- Prometheus metrics
- Logging configurable by env variable
- Add docker deployment
- Better metrics

### 🐛 Bug Fixes

- Kinda works, did some cleanup and small refactoring
- Actix web 3 introduced a dependency mismatch of tokio, upgrade to 4 beta.
- Notification title count
- Send high priority notifications
- Correct rejected array
- Allow configuring bind_ip, default to 127.0.0.1
- *(ci)* Add missing build step for building release container images

### 🚜 Refactor

- Clean up some stuff
- Moved some methods to PushNotification struct
- Logs

### 📚 Documentation

- Add LICENSE
- Update name and authors
- Add example service and proxy readme
- Update readme
- Add code owners

### CI

- Add .gitlab-ci.yml
- Build amd64 release in CI
- Add aarch64 build
- Build fix

<!-- generated by git-cliff -->
