# Kobo Wi-Fi Remote 0.3.0

Kobo Wi-Fi Remote is a remote control/page turner for Kobo e-readers. It installs a server on your device that can record and replay touchscreen inputs via a web interface. It is **not** safe to use on public Wi-Fi networks.

## Supported Devices

It should work on any Kobo e-reader released between 2011 and 2023, but is only actively tested on a Glo and Sage with recent firmware.

If the latest version doesn't work on your device, try [version 0.1.1](https://github.com/sublipri/kobo-wifi-remote/releases/tag/v0.1.1). That's more generic in how it functions, but is slower and has fewer features.

## Features

- Turn pages from a web browser with a simple interface designed for smartphones and users with limited dexterity.
- Keyboard shortcuts for use with a computer.
- Compatible with the [KoboPageTurner](https://github.com/tylpk1216/KoboPageTurner) Android app (supports some Bluetooth devices or the volume keys).
- Custom actions (e.g. adjust the brightness).
- A [NickelMenu](https://pgaskin.net/NickelMenu/) entry and [KOReader](https://koreader.rocks/) plugin for toggling the server.
- Take screenshots using a web browser or NickelMenu.
- Trigger actions with a GET request -- use with a [DIY hardware remote](https://www.mobileread.com/forums/showpost.php?p=4351236&postcount=28) or your [smart watch](https://www.mobileread.com/forums/showpost.php?p=4376646&postcount=30)
- Turn pages automatically at a set interval.

There are some screenshots of the web interface in the thread on [MobileRead](https://www.mobileread.com/forums/showthread.php?t=355368).

### Experimental Features

- Perform input anywhere on the e-reader's screen by using a phone as a trackpad or with a mouse & keyboard (see [below](#arbitrary-input-aka-trackpad-mode)).
- Trigger actions with voice commands. This only works on desktop and perhaps rooted Android devices and requires a Chromium-based browser. See [this comment](https://github.com/sublipri/kobo-wifi-remote/issues/1#issuecomment-2044426815) and the feature's page for details.

## Installation

1. Download the [latest release](https://github.com/sublipri/kobo-wifi-remote/releases/download/v0.3.0/KoboWiFiRemote-0.3.0.zip) and extract the `.zip` file.
1. Connect your e-reader to your computer with a USB cable and browse its storage.
1. Set your computer to show hidden files.
1. Copy the `KoboRoot.tgz` file into the hidden `.kobo` directory on your e-reader.
1. Safely eject your e-reader then disconnect the USB cable. Don't touch the power button.
1. Wait for your e-reader to automatically install the package and reboot itself.
1. Connect your e-reader to your Wi-Fi network. If using a smartphone as the remote, you can use the phone's hotspot.
1. Once connected, the e-reader's IP address will be displayed on screen (also in `More > Settings > Device Information`).
1. Enter the IP address in a web browser and go through the initial setup.

**Warning:** installing this will enable Kobo's ForceWifiOn setting, which drains the battery quicker. You can still manually disable Wi-Fi when not using the remote if this is a concern, and ForceWifiOn can be disabled in the web interface (under Developer Settings).

## Uninstallation

1. Extract the `Uninstaller.zip` file included with the release.
1. Follow steps 2-6 of the installation process.

If you have shell access you can also just run `/opt/wifiremote/bin/wifiremote uninstall` (use `--dry-run` to check what will be removed).

## Troubleshooting/Reporting Bugs

Scroll to the bottom of the homepage (at your e-reader's IP address) and press Troubleshooting. There you'll find instructions for generating a log file and reporting issues.

## Configuration

There are two config files that live in `.adds/wifiremote/` by default. The `user-config.toml` can be edited via the `Edit Config` button on the home screen. This method is recommended as it will validate any changes. You can modify the colors of the buttons, change settings for some features, and disable or re-order the buttons on the home screen. Enabling the `propmpt_fullscreen` option helps to prevent accidentally navigating away from the page, but it probably won't work on iPhones due to browser limitations. Most settings in `app-config.toml` should not be changed by users, and must be edited manually with a text editor. Of note are `allow_remote_restart` (default: `true`) and `allow_remote_exit` (default: `false`) for enabling the `/exit` and `/restart` endpoints.

## Arbitrary Input AKA Trackpad Mode

This experimental feature allows you to perform input anywhere on the e-reader with a mouse and keyboard or by using a phone's touchscreen as a trackpad. It's intended for when the e-reader is mounted somewhere a user is unable to easily reach, so they can still e.g. use the dictionary. You can enable it for the page turner and remote control in their respective sections of the user config file. Then it can be started by long-pressing anywhere on the screen, or with a keyboard (default: `KeyE`). It can also be started more efficiently by swiping in the direction you want the cursor to move, but this is disabled by default since it might not work nicely on all browsers. If starting with a swipe, you'll probably want to enable `prompt_fullscreen`, and, depending on the browser, `swipe_prevent_default`.

It works by drawing a cursor on the e-reader's screen using [FBInk](https://github.com/NiLuJe/FBInk). You can change how often the cursor updates with the `cursor_min_refresh`, `move_send_wait` and `final_move_send_delay` millisecond values. The defaults are fairly conservative for broad compatibility.  On some devices you could reduce them for a more responsive cursor (at the cost of higher CPU usage). Disabling `reload_background_after_input` might also improve responsiveness, but can cause visual artifacts on the screen.

This feature might not work properly on all devices or in all rotations. If input is triggered in a different place to the cursor, the right combination of the `override` settings might make it work properly for a specific rotation.

## Building

Requires [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html), [cross](https://github.com/cross-rs/cross/) and coreutils

Create a `KoboRoot.tgz`:

`./build.sh`

Skip the `KoboRoot.tgz`, transfer the built files to a Kobo via [rsync/ssh](https://www.mobileread.com/forums/showthread.php?t=254214), and restart the server:

`KOBO_HOST=192.168.1.10 ./build.sh deploy`

Create a `KoboRoot.tgz`, copy it with scp, and reboot the device:

`KOBO_HOST=192.168.1.10 ./build.sh install`
