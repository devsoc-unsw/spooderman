#!/bin/bash

if ! command -v cargo-watch &> /dev/null; then
    echo "Error: cargo-watch is not installed. Please install it using 'cargo install cargo-watch'."
    exit 1
fi

# Run cargo watch
cargo watch -q -c -w src/ -x 'run -q'