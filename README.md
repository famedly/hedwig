# Matrix Hedwig

[![pipeline status][badge-pipeline-img]][badge-pipeline-url]
[![coverage report][badge-coverage-img]][badge-coverage-url]

[badge-coverage-img]: https://gitlab.com/famedly/company/backend/services/hedwig/badges/main/coverage.svg
[badge-coverage-url]: https://gitlab.com/famedly/company/backend/services/hedwig/-/commits/main
[badge-pipeline-img]: https://gitlab.com/famedly/company/backend/services/hedwig/badges/main/pipeline.svg
[badge-pipeline-url]: https://gitlab.com/famedly/company/backend/services/hedwig/-/commits/main



This is a dead simple Push Gateway for a [Matrix.org](https://matrix.org) application. It implements the [Matrix Push Notification API r0.1.1](https://matrix.org/docs/spec/push_gateway/r0.1.1) and supports [Firebase Cloud Messaging](https://firebase.google.com/docs/cloud-messaging/) only with iOS and Android being supported. If you need to push to iOS be sure to add the appropriate push key or certificate to your FCM project.

## Features:
- Implements the `POST /_matrix/push/v1/notify` endpoint
- Forwards notifications from the format `event_id_only`
- Returns invalid push keys in the `rejected` response field
- Health status endpoint at `GET /health`
- Version endpoint at `GET /version`
- Prometheus metrics at `GET /metrics`


This project name aged badly, trans rights are human rights!

## Getting Started

### Hedwig configuration:

Please reference `config.sample.yaml` for which settings can be set. The configuration file needs to be named `config.yaml`
The `fcm_service_account_token_path` setting needs to point to an FCM service account token json file.  
The `fcm_push_max_retries` setting specifies how many attempts at pushing a notification to a device should be made before giving up and reporting the push key as dead.

To output to stdout instead of files, remove the `file_output` section from the `config.yaml` file.

There is partial support for the proposed delayed push MSC which is defined [here](https://github.com/matrix-org/matrix-spec-proposals/pull/3359), a maximum jitter delay can be specified with the `max_jitter_delay` setting. Set to 0.0 to disable the jitter completely.

In low traffic situations it might be advisable to disable jitter as it can lead to messages being sent out of order.

### On app side:

Example valid pusher set request (to homeserver, the homeserver will then talk to hedwig whenever there is a notification):  
* `/_matrix/client/v3/pushers/set`:
```json 
{
  "app_display_name": "Aweseome matrix client!",

  // Deprecated: {APP_ID}.data_message is equivalent to setting data-message: "android" in data (keep app_id in hedwig config without the .data_message); This is due to removal, do not rely on it staying around!
  "app_id": "app.id.from.cfg", 
  "append": false,
  "data": {
    "format": "event_id_only",
    "url": "https://your-awesome-pusher.example/_matrix/push/v1/notify",
    "data_message": null | "android" | "ios" // Optional!
  },
  "device_display_name": "🦊phone",
  "kind": "http",
  "lang": "en",
  "profile_tag": "magic-profile-tag",
  "pushkey": "🏰🦊🔒"
}
```

This will result in an FCM notification being sent to the device with the notification request in a `key from json` -> `content from json as string` format.

* Example request sent from hedwig to FCM for an android notification:
```json
{
   "token":"Android",
   "data":{
      "content":"null",
      "counts":"{\"unread\":1337,\"missed_calls\":null}",
      "devices":"[{\"app_id\":\"com.famedly.🦊\",\"pushkey\":\"Android\",\"pushkey_ts\":1655896032,\"data\":{\"data_message\":\"android\",\"format\":\"event_id_only\"},\"tweaks\":null}]", 
      "prio":"\"high\"",
      "room_id":"owo"
   },
   "android":{
      "priority":"high",
      "direct_boot_ok":false
   }
}
```

* Example request to FCM for an iOS notification:
```json
{
   "token":"IoS",
   "data":{
      "content":"null",
      "counts":"{\"unread\":1337,\"missed_calls\":null}",
      "devices":"[{\"app_id\":\"com.famedly.🦊\",\"pushkey\":\"IoS\",\"pushkey_ts\":1655896032,\"data\":{\"data_message\":\"ios\",\"format\":\"event_id_only\"},\"tweaks\":null}]",
      "prio":"\"high\"",
      "room_id":"owo"
   },
   "notification":{
      "title":"🦊 1337 🦊",
      "body":"read the notification pls :c"
   },
   "apns":{
      "headers":{
         "apns-priority":"5",
         "apns-push-type":"background"
      },
      "payload":{
         "aps":{
            "badge":1337,
            "mutable-content":1,
            "sound":"default"
         }
      }
   }
}
```

* Example request to FCM for a notification without having specified the device type (no message content can be displayed):

```json
{
   "token":"Generic",
   "notification":{
      "title":"🦊 1337 🦊",
      "body":"read the notification pls :c"
   },
   "android":{
      "priority":"high",
      "notification":{
         "icon":"notifications_icon",
         "sound":"default",
         "tag":"org.matrix.default_notification",
         "click_action":"FLUTTER_NOTIFICATION_CLICK",
         "channel_id":"org.matrix.app.message"
      },
      "direct_boot_ok":false
   },
   "apns":{
      "headers":{
         "apns-priority":"10"
      },
      "payload":{
         "aps":{
            "badge":1337,
            "sound":"default"
         }
      }
   }
}
```


## Lints

We have plenty of lints in `lints.toml` that we use. Cargo currently does not natively support an extra file for lints, so we use `cargo-lints`. To check everything with our lints, run this locally:

```sh
cargo lints clippy --workspace --all-targets
```

and this in your IDE:

```sh
cargo lints clippy --workspace --all-targets --message-format=json
```

A few lints are commented out in `lints.toml`. This is because they should not be enabled by default, because e.g. they have false positives. However, they can be very useful sometimes.

## Pre-commit usage

1. If not installed, install with your package manager, or `pip install --user pre-commit`
2. Run `pre-commit autoupdate` to update the pre-commit config to use the newest template
3. Run `pre-commit install` to install the pre-commit hooks to your local environment

---

# Famedly

**This project is part of the source code of Famedly.**

We think that software for healthcare should be open source, so we publish most
parts of our source code at [gitlab.com/famedly](https://gitlab.com/famedly/company).

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of
conduct, and the process for submitting pull requests to us.

For licensing information of this project, have a look at the [LICENSE](LICENSE.md)
file within the repository.

If you compile the open source software that we make available to develop your
own mobile, desktop or embeddable application, and cause that application to
connect to our servers for any purposes, you have to aggree to our Terms of
Service. In short, if you choose to connect to our servers, certain restrictions
apply as follows:

- You agree not to change the way the open source software connects and
  interacts with our servers
- You agree not to weaken any of the security features of the open source software
- You agree not to use the open source software to gather data
- You agree not to use our servers to store data for purposes other than
  the intended and original functionality of the Software
- You acknowledge that you are solely responsible for any and all updates to
  your software

No license is granted to the Famedly trademark and its associated logos, all of
which will continue to be owned exclusively by Famedly GmbH. Any use of the
Famedly trademark and/or its associated logos is expressly prohibited without
the express prior written consent of Famedly GmbH.

For more
information take a look at [Famedly.com](https://famedly.com) or contact
us by [info@famedly.com](mailto:info@famedly.com?subject=[GitLab]%20More%20Information%20)
