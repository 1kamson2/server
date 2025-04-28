#!/bin/bash

readonly RESOURCE_CATALOG="$PWD/resource/"
CONFIG_FILE="$RESOURCE_CATALOG$1"

echo "[INFO] Command should be run in the root project directory"
if [[ -f $CONFIG_FILE ]]; then
  cargo run "$CONFIG_FILE"
else
  echo "[ERROR] No file named: "
  echo "$CONFIG_FILE"
fi
