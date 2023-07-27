#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

# shellcheck source=../../config
. "$CONFIG_FILE"

action=$(echo "$REQUEST_URI" | cut -d "/" -f 2)
action="$(urlencode "$action")"
if "$USE_EVEMU"; then
	evemu-play "$INPUT_DEVICE" <"$EVENTS_DIR/$action.evemu"
else
	cat "$EVENTS_DIR/$action.dat" >"$INPUT_DEVICE"
fi

echo "Content-Type: application/json"
echo ""
echo "{\"action\": \"$action\"}"
