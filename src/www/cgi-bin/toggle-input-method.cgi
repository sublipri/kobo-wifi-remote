#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

# shellcheck source=../../config
. "$CONFIG_FILE"

if "$USE_EVEMU"; then
	sed -i -e "s|USE_EVEMU=true|USE_EVEMU=false|" "$CONFIG_FILE"
	message="Input method changed to <strong>cat</strong>"
	logger -p 6 -t wifiremote-input "Input method changed to cat"
else
	sed -i -e "s|USE_EVEMU=false|USE_EVEMU=true|" "$CONFIG_FILE"
	message="Input method changed to <strong>evemu</strong>"
	logger -p 6 -t wifiremote-input "Input method changed to evemu"
fi

output-html "$message"
