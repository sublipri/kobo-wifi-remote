# Kobo Wi-Fi Remote 0.2.0

Kobo Wi-Fi Remote is a remote control/page turner for Kobo e-readers. It installs a server on your device that can record and replay touchscreen inputs via a web interface. It is **not** safe to use on public Wi-Fi networks.

## Features

- A simple interface designed for smartphones and users with limited dexterity.
- Keyboard shortcuts for use with a computer.
- Take screenshots using a web browser or NickelMenu.
- Probably supports all Kobo e-readers with touch screens (tested on a Glo and Sage running the latest firmware).
- Custom actions (e.g. adjust the brightness).
- Trigger actions with a GET request -- use with a [DIY hardware remote](https://www.mobileread.com/forums/showpost.php?p=4351236&postcount=28) or your [smart watch](https://www.mobileread.com/forums/showpost.php?p=4376646&postcount=30)
- A [NickelMenu](https://pgaskin.net/NickelMenu/) entry and [KOReader](https://koreader.rocks/) plugin for toggling the server.

There are some screenshots in the thread on [MobileRead](https://www.mobileread.com/forums/showthread.php?t=355368).

## Installation

1. Download the [latest release](https://github.com/sublipri/kobo-wifi-remote/releases/latest) and extract the .zip file.
1. Connect your e-reader to your computer with a USB cable and browse its storage.
1. Set your computer to show hidden files.
1. Copy the `KoboRoot.tgz` file to the hidden `.kobo` directory on your e-reader.
1. Safely eject your e-reader then disconnect the USB cable. Don't touch the power button.
1. Wait for your e-reader to automatically install the package and reboot itself.
1. Connect your e-reader to your Wi-Fi network. If using a smartphone as the remote, the phone's hotspot might work best.
1. Once connected, the e-reader's IP address will be displayed on screen (also in `More > Settings > Device Information` if required later).
1. Enter the IP address in a web browser and go through the initial setup.

**Warning:** installing this will enable Kobo's ForceWifiOn setting, which drains the battery quicker. You can still manually disable Wi-Fi when not using the remote if this is a concern, and ForceWifiOn can be disabled in the web interface (under Developer Settings).

## Uninstallation

1. Extract the `Uninstaller.zip` file included with the release.
1. Follow steps 2-6 of the installation process.

If you have shell access you can also just run `/opt/wifiremote/bin/wifiremote uninstall` (use `--dry-run` to check what will be removed).

## Troubleshooting/Reporting Bugs

Scroll to the bottom of the homepage (at your e-reader's IP address) and press Troubleshooting. There you'll find instructions for generating a log file and reporting issues.

## Building

Requires [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html), [cross](https://github.com/cross-rs/cross/) and coreutils

Create a `KoboRoot.tgz`:

`./build.sh`

Skip the `KoboRoot.tgz`, transfer the built files to a Kobo via [rsync/ssh](https://www.mobileread.com/forums/showthread.php?t=254214), and restart the server:

`KOBO_HOST=192.168.1.10 ./build.sh deploy`

Create a `KoboRoot.tgz`, copy it with scp, and reboot the device:

`KOBO_HOST=192.168.1.10 ./build.sh install`
