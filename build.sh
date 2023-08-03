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
cp -r -t ./build/root/"$BIN_DIR" ./bin/evemu-*
cp -r -t ./build/root/"$LICENSE_DIR" ./bin/licenses/*
mkdir ./build/root/"$LICENSE_DIR"/wifiremote
cp -t ./build/root/"$LICENSE_DIR"/wifiremote ./LICENSE
cp -t ./build/root/"$USER_DIR" ./src/config

# create udev rule to start wifiremote when a network device is added
UDEV_RULES="KERNEL==\"eth*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote start\"
KERNEL==\"wlan*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote start\""
echo "$UDEV_RULES" >./build/root/"$UDEV_FILE"
ln -s -r ./build/root/"$UDEV_FILE" ./build/root/"$UDEV_LINK"
UNINSTALL_UDEV_RULES="KERNEL==\"eth*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote uninstall\"
KERNEL==\"wlan*\", ACTION==\"add\", RUN+=\"$BIN_DIR/wifiremote uninstall\""

# create main script that sets up the environment and manages httpd
cat >./build/root/"$MAIN_SCRIPT" <<SHELL
#!/bin/sh
set -u

export KOBO_CONFIG_FILE="/mnt/onboard/.kobo/Kobo/Kobo eReader.conf"
export CONFIG_FILE="$USER_DIR/config"
export PATH="\$PATH:$BIN_DIR:$HTTP_DIR/cgi-bin/lib"
export HTTP_DIR="$HTTP_DIR"
export EVENTS_DIR="$EVENTS_DIR"
export VERSION="$VERSION"
export CSV_FILE="$CSV_FILE"
export DATA_DIR="$DATA_DIR"
export DIR_LIST="$DIR_LIST"
export FILE_LIST="$FILE_LIST"

pid=""

httpd_running() {
	pid=\$(pgrep -f "/bin/busybox httpd.*\$HTTP_DIR")
	if [ "\$pid" != "" ]; then
		return 0
	else
		return 1
	fi
}

start_httpd() {
	if httpd_running; then
		echo "Wi-Fi Remote already running"
	else
		if [ "\$1" = "debug" ]; then
			/bin/busybox httpd -vv -f -h "$HTTP_DIR" -p "$HTTP_PORT"
		else
			/bin/busybox httpd -f -h "$HTTP_DIR" -p "$HTTP_PORT" &
		fi
	fi
}

stop_httpd() {
	if httpd_running; then
		kill "\$pid"
	else
		echo "Wi-Fi Remote not running"
	fi
}

enable_wifiremote() {
	ln -s "$UDEV_FILE" "$UDEV_LINK"
	if ! httpd_running; then
		start_httpd normal
	fi
	echo "Wi-Fi Remote enabled"
}

disable_wifiremote() {
	rm "$UDEV_LINK"
	if httpd_running; then
		stop_httpd
	fi
	echo "Wi-Fi Remote disabled"
}

cmd="\$1"

if [ "\$cmd" = "start" ]; then
	start_httpd normal
elif [ "\$cmd" = "start-debug" ]; then
	start_httpd debug
elif [ "\$cmd" = "stop" ]; then
	stop_httpd
elif [ "\$cmd" = "disable" ]; then
	disable_wifiremote
elif [ "\$cmd" = "enable" ]; then
	enable_wifiremote
elif [ "\$cmd" = "toggle" ]; then
	if httpd_running; then
		disable_wifiremote
	else
		enable_wifiremote
	fi
elif [ "\$cmd" = "uninstall" ]; then
	stop_httpd
	cd /
	directories="\$(cat "$DIR_LIST")"
	logfile="/tmp/uninstall.log"
	rm "\$logfile"
	while read -r filename; do
		rm -v "\$filename" >>"\$logfile" 2>&1
	done <"$FILE_LIST"
	rm -v "$EVENTS_DIR"/* >>"\$logfile" 2>&1
	rmdir -v "$HTTP_DIR"/* >>"\$logfile" 2>&1
	while read -r directory; do
		rmdir -v "\$directory" >>"\$logfile" 2>&1
	done <<-DELETE
	\$directories
	DELETE
fi
SHELL
# Format and catch any syntax errors in above script
shfmt --write ./build/root/"$MAIN_SCRIPT"
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
find . -mindepth 1 -type d | sort -r | sed -E "$exclude" | sed "s|^\.||" >./"$DIR_LIST"
find ./ \( -type f -o -type l \) | sed "s|^\.||" >./"$FILE_LIST"
echo "$CSV_FILE" >>./"$FILE_LIST"
cd ../..

create_tgz() {
	cd ./build/root
	tar --create --gzip --owner root --group root --mtime "$(date -u -Iseconds)" --file ../KoboRoot.tgz ./*
	cd ../..
}

if [ "$1" = 'deploy' ]; then
	rsync -vrlh ./build/root/ root@"$KOBO_HOST":/
	ssh root@"$KOBO_HOST" -- "$MAIN_SCRIPT" start
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