#!/bin/sh

PASS=true
for DIR in $(echo "$@" | xargs -n1 dirname | sort -u); do
    cargo test --workspace --lib -- --nocapture
    if [ "$?" -ne 0 ]; then
        PASS=false
    fi
done

if [ "$PASS" = "false" ]; then
    exit 1
fi
