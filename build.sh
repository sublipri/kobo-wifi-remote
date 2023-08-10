#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

set -e

VERSION=$(grep -m 1 -o '[0-9][0-9.]\+\S*' README.md)
HTTP_PORT=80
UDEV_DIR="/etc/udev/rules.d"
HTTP_DIR="/opt/wifiremote/http"
BIN_DIR="/opt/wifiremote/bin"
LICENSE_DIR="/opt/wifiremote/licenses"
DATA_DIR="/opt/wifiremote/data"
USER_DIR="/mnt/onboard/.adds/wifiremote"
UDEV_FILE="$DATA_DIR/udev.rules"
UDEV_LINK="$UDEV_DIR/98-wifiremote.rules"
CSV_FILE="$DATA_DIR/actions.csv"
EVENTS_DIR="$DATA_DIR/events"
MAIN_SCRIPT="$BIN_DIR/wifiremote"
FILE_LIST="$DATA_DIR/tracked_files"
DIR_LIST="$DATA_DIR/tracked_dirs"

if [ -d ./build ]; then
	rm -r ./build
fi

for d in "$UDEV_DIR" "$HTTP_DIR" "$BIN_DIR" "$LICENSE_DIR" "$EVENTS_DIR" "$USER_DIR"; do
	mkdir -p ./build/root/"$d"
done

cp -r -t ./build/root/"$HTTP_DIR" ./src/www/*
cp -r -t ./build/root/"$BIN_DIR" ./bin/evemu-* ./src/wifiremote.sh
cp -r -t ./build/root/"$LICENSE_DIR" ./bin/licenses/*
mkdir ./build/root/"$LICENSE_DIR"/wifiremote
cp -t ./build/root/"$LICENSE_DIR"/wifiremote ./LICENSE
cp ./src/config ./build/root/"$USER_DIR"/config.new

# create udev rule to start wifiremote when a network device is added
UDEV_RULES="KERNEL==\"eth*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote start\"
KERNEL==\"wlan*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote start\""
echo "$UDEV_RULES" >./build/root/"$UDEV_FILE"
ln -s -r ./build/root/"$UDEV_FILE" ./build/root/"$UDEV_LINK"
UNINSTALL_UDEV_RULES="KERNEL==\"eth*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote uninstall\"
KERNEL==\"wlan*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote uninstall\""

# create wrapper script that sets up the environment and runs main script
cat >./build/root/"$MAIN_SCRIPT" <<SHELL
#!/bin/sh

export KOBO_CONFIG_FILE="/mnt/onboard/.kobo/Kobo/Kobo eReader.conf"
export CONFIG_FILE="$USER_DIR/config"
export PATH="\$PATH:$BIN_DIR:$HTTP_DIR/cgi-bin/lib"
export HTTP_DIR="$HTTP_DIR"
export HTTP_PORT="$HTTP_PORT"
export EVENTS_DIR="$EVENTS_DIR"
export VERSION="$VERSION"
export CSV_FILE="$CSV_FILE"
export DATA_DIR="$DATA_DIR"
export DIR_LIST="$DIR_LIST"
export FILE_LIST="$FILE_LIST"
export UDEV_FILE="$UDEV_FILE"
export UDEV_LINK="$UDEV_LINK"

wifiremote.sh "\$@"
SHELL
chmod +x ./build/root/"$MAIN_SCRIPT"

# create NickelMenu config for toggling wifiremote
mkdir ./build/root/mnt/onboard/.adds/nm
cat >./build/root/mnt/onboard/.adds/nm/wifiremote <<EOF
menu_item :main :Wi-Fi Remote (toggle) :cmd_output :1000:"$MAIN_SCRIPT" toggle
EOF

# create list of files and directories to remove when uninstalling
cd ./build/root
# rmdir will only delete empty directories but we'll exclude some for good measure
exclude='/^\.\/(mnt|etc|mnt\/onboard)$/d'
find . -mindepth 1 -type d | sort -r | sed -E "$exclude" | sed "s|^\.||" >./"$DIR_LIST.new"
find ./ \( -type f -o -type l \) | sed "s|^\.||" | sed 's|\.new$||' | sort >./"$FILE_LIST.new"
echo "$CSV_FILE" >>./"$FILE_LIST.new"
cd ../..

create_tgz() {
	cd ./build/root
	tar --create --gzip --owner root --group root --mtime "$(date -u -Iseconds)" --file ../KoboRoot.tgz ./*
	cd ../..
}

if [ "$1" = 'deploy' ]; then
	rsync -vrlh ./build/root/ root@"$KOBO_HOST":/
	ssh root@"$KOBO_HOST" -- "$MAIN_SCRIPT" restart
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
	rm -r ./build/root
	cd ./build
	zip --test --move "$release_zip" ./*
	cd ..
	mkdir -p ./build/root/"$UDEV_DIR"
	echo "$UNINSTALL_UDEV_RULES" >./build/root/"$UDEV_LINK"
	create_tgz
	cd ./build
	zip --test --move Uninstaller.zip KoboRoot.tgz
	zip --test --move "$release_zip" Uninstaller.zip
	cd ..
	rm -r ./build/root
fi
