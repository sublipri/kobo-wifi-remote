#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

# shellcheck source=../../config
. "$CONFIG_FILE"
old_input_device=$INPUT_DEVICE
invalid_input=true
for e in /dev/input/ev*; do
	if [ "$QUERY_STRING" = "$e" ]; then
		sed -i -e "s|$old_input_device|$e|" "$CONFIG_FILE"
		invalid_input=false
	fi
done

if "$invalid_input"; then
	message="$QUERY_STRING is not a valid input device"
else
	message="Input Device changed to <strong>$QUERY_STRING</strong>"
fi
output-html "$message"
