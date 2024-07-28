# Changelog

## Unreleased

### Added
- A way to configure how rotation is detected on sunxi devices (Sage & Elipsa).

### Fixed
- Landscape rotation detection on the Glo (existing landscape recordings on the Glo will need re-doing)
- The user config being overwritten if it has fields missing when edited from the web interface

## 0.3.1 - 2024-07-18
This release fixes a minor bug and updates FBInk so the latest Kobo models might work.

### Changed
- Updated dependencies

### Fixed
- The configured auto turner default delay not being used

## 0.3.0 - 2024-07-15

This release adds new features with an accessibility focus, fixes bugs, and makes more things user-configurable.

### Added
- User config and app config files
- A rudimentary way to edit the user config from the web interface
- A fullscreen mode for the page turner & remote control to make it harder to accidentally navigate away (disabled by default and might not work on iPhones)
- Turn pages automatically at a set interval
- Experimental features:
    - Perform input anywhere on the e-reader's screen by using a phone as a trackpad or with a mouse & keyboard (might not work on all devices or in all rotations)
    - Trigger actions with voice commands (doesn't work on mobile except for perhaps rooted Android devices)
- `/restart` and `/exit` endpoints for restarting and stopping the server (restart is enabled and exit disabled by default - see app config)

### Changed
- Action playback endpoints will now return a JSON response.
- Custom actions will no longer be optimized by default.
- `/dev/input/` will be used by default instead of `/dev/input/by-path/` (changeable in user config).

### Fixed
- The KoboPageTurner Android app crashing when turning pages
- Action recording and logging not working on the Kobo Aura H2O

## 0.2.0 - 2024-03-07

This release replaces BusyBox httpd + shell scripts with a new backend written in Rust, which along with [bindings](https://github.com/sublipri/fbink-rs) for [FBInk](https://github.com/NiLuJe/FBInk) brings new features and various improvements. Due to the significant changes, you'll need to redo the initial setup after upgrading. You can upgrade in place, but you may wish to uninstall the previous version first to remove lots of no-longer needed files (they'll still be removed if you uninstall in the future).

### Highlights
- Support for multiple rotations
- Faster page turns (feels about twice as fast on a Sage, but not that noticeable on a Glo)
- Take screenshots using the web UI or NickelMenu
- A simpler setup process

### Added
- A KOReader plugin for toggling the server
- `status` and `screenshot` commands to the CLI
- Edit a recorded action's options via the web UI or `.adds/wifiremote/actions.toml`
- Support for toggling a few more developer settings

### Changed
- Action playback endpoints are now prefixed with `/actions/`. Page turning uses `/actions/next-page` and `/actions/prev-page`, but `/left` and `/right` are kept for backwards compatibility.
- ForceWifiOn will be enabled on boot the first time the server runs, instead of requiring manual activation during setup.
- The e-reader's IP address will be displayed on screen when the server first runs until the initial setup is done.
- Single swipes and taps (i.e. page turns) will be optimized to replay quicker than they were recorded.
- Successive actions are now played in a queue with customizable delay, so you can spam page turns and they should all register.
- The page turner and remote control will now display an error if a request fails.
- The `enable` and `disable` CLI commands will no longer start/stop the server unless `--now` is passed.
- The `uninstall` CLI command now has a `--dry-run` flag.

### Fixed
- The server should now work with hardware using KoboPageTurner's [code](https://github.com/tylpk1216/KoboPageTurner/blob/master/ESP8266/SoftAP_No_OTA.ino) without having to add a `/` to the endpoints.

## 0.1.1 - 2023-08-06

### Added
- Debug logging via syslog.
- A troubleshooting page with steps to follow and a link that generates a log file.

### Fixed
- Stopping httpd now works if a child process is running.
- Minor fixes and improvements.

## 0.1.0 - 2023-07-29

Initial release
