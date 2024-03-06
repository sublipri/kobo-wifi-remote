#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

set -e

VERSION=$(grep -m 1 -o '[0-9][0-9.]\+\S*' README.md)
UDEV_DIR="/etc/udev/rules.d"
BIN_DIR="/opt/wifiremote/bin"
DATA_DIR="/opt/wifiremote/data"
USER_DIR="/mnt/onboard/.adds/wifiremote"
UDEV_FILE="$DATA_DIR/udev.rules"
UDEV_LINK="$UDEV_DIR/98-wifiremote.rules"
MAIN_BIN="$BIN_DIR/wifiremote"
FILE_LIST="$DATA_DIR/tracked_files"
DIR_LIST="$DATA_DIR/tracked_dirs"
TARGET="armv7-unknown-linux-musleabihf"

if [ -d ./build ]; then
	rm -r ./build
fi

if [ "$1" = 'release' ]; then
	# Trims about 2MB off the binary size but much slower to build
	PROFILE="release-minsized"
else
	# Debug builds are too big for the rootfs so use release for testing
	PROFILE="release"
fi

cross build --profile "$PROFILE" --target "$TARGET"

for d in "$UDEV_DIR" "$DATA_DIR" "$BIN_DIR" "$USER_DIR"; do
	mkdir -p ./build/root/"$d"
done

cp ./target/"$TARGET"/"$PROFILE"/kobo-wifi-remote ./build/root/"$MAIN_BIN"

# create udev rule to start wifiremote when a network device is added
UDEV_RULES="KERNEL==\"eth*\", ACTION==\"add\", RUN+=\"$MAIN_BIN start\"
KERNEL==\"wlan*\", ACTION==\"add\", RUN+=\"$MAIN_BIN start\""
echo "$UDEV_RULES" >./build/root/"$UDEV_FILE"
ln -s -r ./build/root/"$UDEV_FILE" ./build/root/"$UDEV_LINK"
UNINSTALL_UDEV_RULES="KERNEL==\"eth*\", ACTION==\"add\", RUN+=\"$MAIN_BIN uninstall\"
KERNEL==\"wlan*\", ACTION==\"add\", RUN+=\"$MAIN_BIN uninstall\""

# create NickelMenu config for toggling wifiremote
mkdir ./build/root/mnt/onboard/.adds/nm
cat >./build/root/mnt/onboard/.adds/nm/wifiremote <<EOF
menu_item :main :Wi-Fi Remote (toggle) :cmd_output :1000:"$MAIN_BIN" toggle
menu_item :main :Screenshot (1s delay) :cmd_spawn :quiet:"$MAIN_BIN" screenshot --fbink --delay 1
menu_item :reader :Screenshot (2s delay) :cmd_spawn :quiet:"$MAIN_BIN" screenshot --fbink --delay 2
# menu_item :main :Wi-Fi Remote (enable) :cmd_output :1000:"$MAIN_BIN" enable --now
# menu_item :main :Wi-Fi Remote (disable) :cmd_output :1000:"$MAIN_BIN" disable --now
# menu_item :main :Wi-Fi Remote (status) :cmd_output :1000:"$MAIN_BIN" status
# menu_item :main :Wi-Fi Remote (restart) :cmd_output :1000:"$MAIN_BIN" restart
EOF

# create list of files and directories to remove when uninstalling
(
	cd ./build/root
	# exclude some important system directories for good measure
	exclude='/^\.\/(mnt|etc|mnt\/onboard)$/d'
	find . -mindepth 1 -type d | sort -r | sed -E "$exclude" | sed "s|^\.||" >./"$DIR_LIST.new"
	find ./ \( -type f -o -type l \) | sed "s|^\.||" | sed 's|\.new$||' | sort >./"$FILE_LIST.new"
)

create_tgz() {
	(
		cd ./build/root
		tar --create --gzip --owner root --group root --mtime "$(date -u -Iseconds)" --file ../KoboRoot.tgz ./*
	)
}

if [ "$1" = 'deploy' ]; then
	rsync -vrlh --progress ./build/root/ root@"$KOBO_HOST":/
	ssh root@"$KOBO_HOST" -- "$MAIN_BIN" restart
	rm -r ./build
else
	create_tgz
fi

if [ "$1" = 'install' ]; then
	scp ./build/KoboRoot.tgz root@"$KOBO_HOST":/mnt/onboard/.kobo/KoboRoot.tgz
	ssh root@"$KOBO_HOST" reboot
elif [ "$1" = 'release' ]; then
	release_zip="KoboWiFiRemote-$VERSION.zip"
	pandoc -o ./build/README.html README.md
	cp -t ./build/ ./LICENSE
	rm -r ./build/root
	(cd ./build && zip --test --move "$release_zip" ./*)
	mkdir -p ./build/root/"$UDEV_DIR"
	echo "$UNINSTALL_UDEV_RULES" >./build/root/"$UDEV_LINK"
	create_tgz
	(
		cd ./build
		zip --test --move Uninstaller.zip KoboRoot.tgz
		zip --test --move "$release_zip" Uninstaller.zip
	)
	rm -r ./build/root
fi
