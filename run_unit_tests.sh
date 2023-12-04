#!/bin/sh

PASS=true
cargo test --workspace -- --nocapture
if [ "$?" -ne 0 ]; then
    PASS=false
fi
if [ "$PASS" = "false" ]; then
    exit 1
fi
