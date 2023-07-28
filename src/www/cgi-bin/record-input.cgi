#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

# shellcheck source=../../config
. "$CONFIG_FILE"

QUERY_STRING=$(echo "$QUERY_STRING" | sed -e "s/\*/%2A/g" -e "s/\./%2E/g" -e "s/+/%20/g")
get_var() {
	echo "$QUERY_STRING" | sed -n "s|^.*${1}=\([^&]*\).*$|\1|p"
}

path_segment=$(get_var "path-segment")
display_name=$(get_var "display-name")
sort_value=$(get_var "sort-value")
keyboard_shortcut=$(get_var "keyboard-shortcut")
duration=$(get_var "duration")

if [ "$duration" = "" ] || ! echo "$duration" | grep -q -e '^[0-9]*$'; then
	duration="$CAPTURE_DURATION"
fi
if [ "$path_segment" = "" ]; then
	path_segment="$display_name"
fi
if [ "$sort_value" = "" ]; then
	sort_value="$display_name"
fi

action_dir="$HTTP_DIR/$path_segment"

# query string input validation
valid_query_string=false
component_regex='^[0-9a-zA-Z%_-]*$'
if [ "$display_name" = "" ]; then
	message="A Display name is required"
elif ! echo "$display_name" | grep -q -e "$component_regex"; then
	message="Display name must be an encoded URI component"
elif ! echo "$sort_value" | grep -q -e "$component_regex"; then
	message="Sort value must be an encoded URI component"
elif ! echo "$path_segment" | grep -q -e "$component_regex"; then
	message="URL path segment must be an encoded URI component"
elif echo "$path_segment" | grep -q -e '%2F'; then
	message="URL path segment must not contain a slash"
elif ! echo "$keyboard_shortcut" | grep -q -e '^[0-9a-zA-Z]*$'; then
	message="Keyboard shortcut must only contain alphanumeric characters"
elif grep -q -e "^$action_dir" <"$DIR_LIST"; then
	message="<strong>$path_segment</strong> is a reserved URL path segment"
elif [ -d "$action_dir" ] && ! grep -q -e "^$path_segment,$display_name," <"$CSV_FILE"; then
	message="URL path segment <strong>$path_segment</strong> already in use"
else
	valid_query_string=true
fi

if ! "$valid_query_string"; then
	output-html "Error: $message"
	exit
fi

# Record touchscreen input
touchscreen_input_detected=false
tmp_recording="/tmp/action"
if "$USE_EVEMU"; then
	action_file="$EVENTS_DIR/$path_segment.evemu"
	timeout "$duration" evemu-record "$INPUT_DEVICE" >"$tmp_recording"
	if grep -q -e '^E' <"$tmp_recording"; then
		touchscreen_input_detected=true
	fi
else
	action_file="$EVENTS_DIR/$path_segment.dat"
	timeout "$duration" cat "$INPUT_DEVICE" >"$tmp_recording"
	if [ -s "$tmp_recording" ]; then
		touchscreen_input_detected=true
	fi
fi

if "$touchscreen_input_detected"; then
	mv "$tmp_recording" "$action_file"
	name="$(httpd -d "$display_name")"
	message="Successfully recorded input for <strong>$name.</strong>"
	mkdir -p "$action_dir"
	csv_line="$path_segment,$display_name,$sort_value,$keyboard_shortcut"
	if grep -q -e "^$path_segment," <"$CSV_FILE"; then
		sed -i -e "/^$path_segment,/c$csv_line" "$CSV_FILE"
	else
		echo "$csv_line" >>"$CSV_FILE"
	fi
else
	rm "$tmp_recording"
	javascript="<script type='module'>import { setup } from '/js/alert-recording.js'; setup($duration);</script>"
	message="<p>No input detected.</p> <p><a href=\"javascript:window.location.reload()\"><button class=records-input>Try Again</button></a>"
fi

output-html "$message" "$javascript"
