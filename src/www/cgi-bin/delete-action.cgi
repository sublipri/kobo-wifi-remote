#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

QUERY_STRING=$(echo "$QUERY_STRING" | sed -e "s/\*/%2A/g" -e "s/\./%2E/g" -e "s/+/%20/g")
get_var() {
	echo "$QUERY_STRING" | sed -n "s|^.*${1}=\([^&]*\).*$|\1|p"
}

path_segment=$(get_var "path-segment")
if [ "$path_segment" = "" ]; then
	message="No action selected."
elif grep -q "^$path_segment," <"$CSV_FILE"; then
	name=$(httpd -d "$path_segment")
	if [ -x "$EVENTS_DIR/$path_segment.evemu" ]; then
		rm "$EVENTS_DIR/$path_segment.evemu"
	fi
	if [ -x "$EVENTS_DIR/$path_segment.dat" ]; then
		rm "$EVENTS_DIR/$path_segment.dat"
	fi
	rmdir "$HTTP_DIR/$path_segment"
	sed -i -e "/^$path_segment,/d" "$CSV_FILE"
	message="$name deleted."
else
	message="$name does not exist."
fi

output-html "$message"
