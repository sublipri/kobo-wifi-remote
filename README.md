# Kobo Wi-Fi Remote 0.1.1

Kobo Wi-Fi Remote is a web-based remote control/page turner for Kobo e-readers. It uses the BusyBox HTTP server included with Kobo's stock firmware to serve a website that can record and playback touchscreen inputs via CGI shell scripts. It is **not** safe to use on public Wi-Fi networks.

## Features

- A simple interface designed for smartphones and users with limited dexterity.
- Keyboard shortcuts for use with a computer.
- Probably supports all Kobo e-readers with Wi-Fi (tested on a Glo and Sage running the latest firmware).
- Custom actions (e.g. adjust the brightness).
- Trigger actions with a GET request. By default it uses the same endpoints as [KoboPageTurner](https://github.com/tylpk1216/KoboPageTurner) for compatibility with existing apps and hardware.
- A [NickelMenu](https://pgaskin.net/NickelMenu/) toggle for enabling and disabling the server.
- Should work with third-party document readers such as [KOReader](https://koreader.rocks/).

There are some screenshots in the thread on [MobileRead](https://www.mobileread.com/forums/showthread.php?t=355368).

## Installation

1. Download the [latest release](https://github.com/sublipri/kobo-wifi-remote/releases/latest) and extract the .zip file.
1. Connect your e-reader to your computer with a USB cable and mount it as an external storage device.
1. Set your computer to show hidden files.
1. Copy the `KoboRoot.tgz` file to the hidden `.kobo` directory on your e-reader.
1. Safely eject your e-reader then disconnect the USB cable. Don't touch the power button.
1. Wait for your e-reader to automatically install the package and reboot itself.
1. Connect your e-reader to your Wi-Fi network.
1. From the home screen tap `More > Settings > Device Information` and locate the IP address.
1. Enter the IP address in a web browser and go through the initial setup.

## Uninstallation

1. Extract the `Uninstaller.zip` file included with the release.
1. Follow steps 2-6 of the installation process.

## Troubleshooting/Reporting Bugs

Scroll to the bottom of the homepage (at your e-reader's IP address) and press Troubleshooting. There you'll find instructions for generating a log file and reporting issues.
