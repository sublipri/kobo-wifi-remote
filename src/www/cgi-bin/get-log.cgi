#!/bin/sh

echo "Content-Type: application/x-download"
echo "Content-Disposition: attachment; filename=wifiremote-$(date +'%s').log.gz"
echo ""
logread | grep wifiremote- | gzip -
