#!/bin/sh
# SPDX-FileCopyrightText: 2023 sublipri <sublipri@proton.me>
# SPDX-License-Identifier: GPL-3.0-only

sorted="$(sort -f -k3 -t, "$CSV_FILE")"
html='<form id="delete-action" method="GET" accept-charset="UTF-8" action="/cgi-bin/delete-action.cgi">
  <select form="delete-action" name="path-segment" id="select-action">
  <option value="">--Please select an action--</option>'
while read -r line; do
	name=$(echo "$line" | cut -d, -f2)
	display_name="$(httpd -d "$name")"
	path_segment=$(echo "$line" | cut -d, -f1)
	path_segment="$(httpd -d "$path_segment")"
	html="${html}<option value=\"$path_segment\">$display_name</option>"
done <<EOF
$sorted
EOF
html="${html}</select></p><p><button type=\"submit\">Delete Action</button></form>"
echo "$sorted" | logger -p 7 -t wifiremote-manage
logger -p 7 -t wifiremote-manage "$html"
output-html "$html"
