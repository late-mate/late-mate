#!/bin/bash
set -o errexit -o nounset -o pipefail
cd "`dirname $0`/.."

RUST_BACKTRACE=1 npm run tauri dev