#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

if grep -q -e "^EnableDebugServices=true" <"$KOBO_CONFIG_FILE"; then
	sed -i -e "s/^EnableDebugServices=true/EnableDebugServices=false/" "$KOBO_CONFIG_FILE"
	message="Debug Services disabled. Your Kobo will reboot to apply the changes."
	reboot -d 2 &
else
	message="Debug Services already disabled"
fi

output-html "$message"
