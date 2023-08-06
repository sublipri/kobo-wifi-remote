#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only
# Main script. Must be called by wrapper script that sets up the environment
set -u
# Fail early if environment not set up
echo "$HTTP_DIR" >/dev/null

syslog_pid=$(pgrep -o -f syslogd)
while [ "$syslog_pid" = "" ]; do
	logger -s -p 6 -t wifiremote-main "Syslog not running. Waiting 1 second."
	sleep 1
	syslog_pid=$(pgrep -o -f syslogd)
done

httpd_pid=""
httpd_running() {
	httpd_pid=$(pgrep -o -f "^/bin/busybox httpd.*$HTTP_DIR")
	if [ "$httpd_pid" != "" ]; then
		pgrep -laf "^/bin/busybox httpd.*$HTTP_DIR" | logger -p 7 -t wifiremote-main
		return 0
	else
		return 1
	fi
}

start_httpd() {
	if httpd_running; then
		echo "Wi-Fi Remote already running"
	else
		# If e-reader has just booted log dmesg in case there's useful info.
		# Over time dmesg will fill up with spam that overwrites anything useful.
		uptime="$(cat /proc/uptime)"
		if [ "${uptime%%.*}" -lt 60 ]; then
			dmesg | logger -p 7 -t wifiremote-system
		fi
		tr '\0' '\n' <"/proc/$(pidof -s dbus-daemon)/environ" | sed /^snum/d | logger -p 7 -t wifiremote-system
		df -h | logger -p 7 -t wifiremote-system
		logger -p 6 -t wifiremote-main "Starting Wi-Fi Remote $VERSION"
		printenv | sort | logger -p 7 -t wifiremote-main
		check_config
		# unset unneeded environment variables so they're not logged
		unset UDEV_LOG ACTION SEQNUM IFINDEX DEVPATH SUBSYSTEM INTERFACE
		unset OF_NAME OF_FULLNAME OF_COMPATIBLE_0 OF_TYPE OF_FULLNAME OF_COMPATIBLE_N MODALIAS DRIVER
		unset UDEV_LINK UDEV_FILE
		/bin/busybox httpd -vv -f -h "$HTTP_DIR" -p "$HTTP_PORT" 2>&1 | logger -p 7 -t wifiremote-httpd &
	fi
}

check_config() {
	logger -p 7 -t wifiremote-main -- "--- Contents of $CONFIG_FILE ---"
	if [ -s "$CONFIG_FILE" ]; then
		logger -p 7 -t wifiremote-main <"$CONFIG_FILE"
	fi
	if [ -s "$CONFIG_FILE".new ]; then
		logger -p 7 -t wifiremote-main -- "--- Contents of $CONFIG_FILE.new ---"
		logger -p 7 -t wifiremote-main <"$CONFIG_FILE.new"
		if [ ! -s "$CONFIG_FILE" ]; then
			mv -v "$CONFIG_FILE.new" "$CONFIG_FILE" 2>&1 | logger -p 7 -t wifiremote-main
		else
			while read -r line; do
				prefix=$(echo "$line" | grep -o "^.*=")
				if ! grep -q "$prefix" <"$CONFIG_FILE"; then
					echo "$line" >>"$CONFIG_FILE"
				fi
			done <"$CONFIG_FILE.new"
			rm -v "$CONFIG_FILE.new" 2>&1 | logger -p 7 -t wifiremote-main
			logger -p 7 -t wifiremote-main -- "--- Updated Config File ---"
			logger -p 7 -t wifiremote-main <"$CONFIG_FILE"
		fi
	fi
}

stop_httpd() {
	if httpd_running; then
		logger -p 6 -t wifiremote-main "Stopping Wi-Fi Remote"
		logger -p 7 -t wifiremote-main "Killing PID $httpd_pid"
		kill "$httpd_pid"
	else
		echo "Wi-Fi Remote not running"
	fi
}

enable_wifiremote() {
	ln -s "$UDEV_FILE" "$UDEV_LINK"
	if ! httpd_running; then
		start_httpd
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

cmd="$1"

if [ "$cmd" = "start" ]; then
	start_httpd
elif [ "$cmd" = "stop" ]; then
	stop_httpd
elif [ "$cmd" = "disable" ]; then
	disable_wifiremote
elif [ "$cmd" = "enable" ]; then
	enable_wifiremote
elif [ "$cmd" = "toggle" ]; then
	if httpd_running; then
		disable_wifiremote
	else
		enable_wifiremote
	fi
elif [ "$cmd" = "uninstall" ]; then
	stop_httpd
	cd /
	directories="$(cat "$DIR_LIST")"
	while read -r filename; do
		rm -v "$filename" 2>&1 | logger -p 6 -t wifiremote-uninstall
	done <"$FILE_LIST"
	rm -v "$EVENTS_DIR"/* 2>&1 | logger -p 6 -t wifiremote-uninstall
	rmdir -v "$HTTP_DIR"/* 2>&1 | logger -p 6 -t wifiremote-uninstall
	while read -r directory; do
		rmdir -v "$directory" 2>&1 | logger -p 6 -t wifiremote-uninstall
	done <<-DELETE
		$directories
	DELETE
fi
