#!/bin/bash
# Optional, one-time per Flex: install a developer CA so sideloads stop
# prompting "Allow unsafe manager" on every load. ledgerctl manages its own
# CA keypair (~/.ledgerwallet). Device attached via usbipd, unlocked, on the
# dashboard; approve the certificate on the device.
set -e
source "$(dirname "$0")/env.sh"
ledgerctl install-ca "presse-dev"
