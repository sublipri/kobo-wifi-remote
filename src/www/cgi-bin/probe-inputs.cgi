#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

# shellcheck source=../../config
. "$CONFIG_FILE"
configured_input=$INPUT_DEVICE

mkdir -p /tmp/eventprobe
for event in /dev/input/ev*; do
	timeout "$CAPTURE_DURATION" cat "$event" >/tmp/eventprobe/"$(basename "$event")" &
done
logger -p 7 -t wifiremote-input </proc/bus/input/devices
printenv | sort | logger -p 7 -t wifiremote-input
sleep "$CAPTURE_DURATION"

if "$FIRST_RUN"; then
	for device in /sys/class/input/event*/device; do
		if grep -i touch <"$device"/name; then
			configured_input=/dev/input/$(basename "$device"/event*)
			sed -i -e "s|$INPUT_DEVICE|$configured_input|" "$CONFIG_FILE"
			name=$(cat "$device"/name)
			logger -p 6 -t wifiremote-input "Selecting $configured_input ($name) as default device"
			break
		fi
	done
	sed -i -e "s|FIRST_RUN=true|FIRST_RUN=false|" "$CONFIG_FILE"
fi

input_detected=false
configured_input_detected=false
probe_log="<div style=\"text-align: left\"><p>The following input devices were probed:</p>"
for device in /sys/class/input/event*/device; do
	event=$(basename "$device"/event*)
	event_path=/dev/input/"$event"
	device_name=$(cat "$device"/name)
	probe_log="${probe_log}<p><strong>$event_path:</strong>"
	probe_log="${probe_log}<br>Name: $device_name"
	if [ "$configured_input" = "$event_path" ]; then
		probe_log="${probe_log}<br>Selected Device: <strong>Yes</strong>"
	else
		probe_log="${probe_log}<br>Selected Device: No"
	fi

	if [ -s "/tmp/eventprobe/$event" ]; then
		probe_log="${probe_log}<br>Input Detected: <strong>Yes</strong>"
		input_detected=true
		logger -p 6 -t wifiremote-input "Input detected on $event_path"
		if [ "$configured_input" != "$event_path" ]; then
			url="/cgi-bin/set-input-device.cgi?$event_path"
			probe_log="${probe_log}<p><a href=\"$url\"><button>Select $device_name</button></a></p>"
		else
			configured_input_detected=true
		fi
	else
		probe_log="${probe_log}<br>Input Detected: No"
	fi
	probe_log="${probe_log}</p>"
done
probe_log="${probe_log}</div>"

if ! "$input_detected"; then
	logger -p 6 -t wifiremote-input "No input detected"
	html="<p><strong>No input detected</strong>. 
	Make sure to tap your e-reader's screen while the page loads.</p>
	<div style=\"text-align:center\"> 
	<p><a href=\"javascript:window.location.reload()\"><button class='records-input'>Try Again</button></a></p></div>"
elif "$configured_input_detected"; then
	html="<p><strong>Everything okay!</strong> 
	Input detected on the selected device.</p>"
else
	html="Input detected on a different device than selected. 
	Press the button below to change device."
fi

html="${html}</p><p>$probe_log"

show_switch_inputs=false
if ! "$USE_EVEMU"; then
	show_switch_inputs=true
	button_text="Switch Input Method"
	html="${html}<p>Current input method is <strong>cat</strong>, 
	which can't record pauses in actions.
	To enable recording complex custom actions, switch to <strong>evemu</strong>.
	You will need to re-record any existing actions.</p>"
elif ! "$input_detected"; then
	show_switch_inputs=true
	button_text="Switch Input Method"
	html="${html}<p>Current input method is <strong>evemu</strong>. 
	This allows recording complex actions with pauses but might not work with some devices. 
	If you continue to have issues and only require basic page turning, try switching to <strong>cat</strong>.</p>"
fi

if "$show_switch_inputs"; then
	html="${html}<p><a href=\"/cgi-bin/toggle-input-method.cgi\">
	<button>${button_text}</button></a></p>"
fi

rm -r /tmp/eventprobe
javascript="<script type='module'>import { setup } from '/js/alert-recording.js'; setup($CAPTURE_DURATION);</script>"
output-html "$html" "$javascript"
