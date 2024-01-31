#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

printenv | sort | logger -p 7 -t wifiremote-control
ls -lA --group-directories-first "$HTTP_DIR" | logger -p 7 -t wifiremote-control
if [ ! -s "$CSV_FILE" ]; then
	logger -p 6 -t wifiremote-control "CSV file empty"
	output-html "No actions were found. Have you performed the initial setup?"
	exit
else
	logger -p 7 -t wifiremote-control "CSV Contents:"
	logger -p 7 -t wifiremote-control <"$CSV_FILE"
fi
# shellcheck source=../../config
. "$CONFIG_FILE"
COLOR1="#33b249"
COLOR2="#5783db"

html='Content-Type: text/html
Cache-Control: no-cache

<!doctype html>
<html>
<head>
<meta name="viewport" content="width=device-width", initial-scale="1", charset="UTF-8">
<link href="/styles/main.css" rel="stylesheet" />
<link href="/styles/remote.css" rel="stylesheet" />
<title>Kobo Wi-Fi Remote</title>
</head>
<body style="margin: 0; padding: 0">
<div class="button-container">'
javascript='window.addEventListener("keydown",(e=>{switch(e.code){'
sorted="$(sort -f -k3 -t, "$CSV_FILE")"
i=0
logger -p 7 -t wifiremote-control "Sorted CSV Contents:"
echo "$sorted" | logger -p 7 -t wifiremote-control
while read -r line; do
	name=$(echo "$line" | cut -d, -f2)
	name=$(httpd -d "$name")
	path_segment=$(echo "$line" | cut -d, -f1)
	shortcut=$(echo "$line" | cut -d, -f4)
	btn_id="btn$i"
	if [ $((i % 2)) -eq 0 ]; then
		color="$COLOR1"
	else
		color="$COLOR2"
	fi

	if [ "$shortcut" != "" ]; then
		click_btn="const $btn_id = document.getElementById('$btn_id'); $btn_id.click()"
		javascript="${javascript}case'$shortcut':$click_btn;break;"
	else
		shortcut="None"
	fi

	html="$html
	<button 
	class='remote-button colored-button' 
	type='button'
	id='$btn_id' 
	style='background-color: $color' 
	title='$name (Shortcut: $shortcut)'
	onclick='fetch(\"/$path_segment/\")'
	>
	$name
	</button>"
	i=$((i + 1))
done <<EOF
$sorted
EOF
javascript="${javascript}default:break;}}),!0);"
logger -p 7 -t wifiremote-control "Remote HTML:"
logger -p 7 -t wifiremote-control "$html"
logger -p 7 -t wifiremote-control "Remote Javascript:"
logger -p 7 -t wifiremote-control "$javascript"
echo "$html"
echo "</div>"
echo "</body>"
echo "<script>"
echo "$javascript"
echo "</script>"
echo "<script src='/js/colored-buttons.js'></script>"
echo "</html>"
