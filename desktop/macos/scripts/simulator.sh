#!/bin/sh
# Prints the UDID of the first iPhone on the newest installed iOS runtime, so
# the test scripts work on whichever simulators a machine or runner image ships.
set -eu

xcrun simctl list devices available --json | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
runtimes = sorted(
    (key for key in devices if ".SimRuntime.iOS-" in key),
    key=lambda key: [int(part) for part in key.rsplit("iOS-", 1)[1].split("-")],
)
for runtime in reversed(runtimes):
    for device in devices[runtime]:
        if device["name"].startswith("iPhone"):
            print(device["udid"])
            sys.exit(0)
sys.exit("no iPhone simulator is installed")
'
