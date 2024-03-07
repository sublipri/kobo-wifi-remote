# Changelog

## 0.2.0

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
