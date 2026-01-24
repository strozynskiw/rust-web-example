#!/bin/bash
# Wrapper script to create a user
# Usage: ./scripts/create_user.sh <username> <email> <password> [role]

if [ $# -lt 3 ] || [ $# -gt 4 ]; then
    echo "Usage: $0 <username> <email> <password> [role]"
    echo ""
    echo "Arguments:"
    echo "  username - User's username"
    echo "  email    - User's email address"
    echo "  password - User's password (min 8 characters)"
    echo "  role     - Optional: 'admin' or 'user' (default: 'admin')"
    echo ""
    echo "Examples:"
    echo "  $0 admin admin@example.com SecurePass123         # Create admin"
    echo "  $0 john john@example.com SecurePass123 user      # Create user"
    exit 1
fi

USERNAME=$1
EMAIL=$2
PASSWORD=$3
ROLE=${4:-admin}

cd "$(dirname "$0")/.." || exit 1
cargo run --bin create_admin -- "$USERNAME" "$EMAIL" "$PASSWORD" "$ROLE"
