#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"

# Find the Stalwart container
CONTAINER=$(docker ps --format '{{.Names}}' | grep -i stalwart | head -1)

if [ -z "$CONTAINER" ]; then
    echo "ERROR: No running Stalwart container found"
    exit 1
fi

echo "Deploying forked webadmin to container: $CONTAINER"
echo "Source: $DIST_DIR"

# Backup current webadmin
echo "Backing up current webadmin..."
docker exec "$CONTAINER" sh -c "cp -r /opt/stalwart/webadmin /opt/stalwart/webadmin.bak" 2>/dev/null || true

# Copy new webadmin files
echo "Copying new webadmin files..."
docker cp "$DIST_DIR/." "$CONTAINER:/opt/stalwart/webadmin/"

echo "Done! Refresh the Stalwart admin panel in your browser."
echo ""
echo "To rollback: docker exec $CONTAINER sh -c 'rm -rf /opt/stalwart/webadmin && mv /opt/stalwart/webadmin.bak /opt/stalwart/webadmin'"
