#!/bin/bash
set -euo pipefail
# Wrapper script to create a user
# Usage: ./scripts/create_user.sh <username> <email> [admin|user] [--password-stdin]

if [ $# -lt 2 ] || [ $# -gt 4 ]; then
    echo "Usage: $0 <username> <email> [admin|user] [--password-stdin]" >&2
    echo "The password is read securely from a prompt, never from arguments." >&2
    exit 1
fi

cd "$(dirname "$0")/.." || exit 1
exec cargo run --locked --bin create_admin -- "$@"
