#!/bin/bash
# Script to pre-load default Docker images into the registry

set -e

MANAGER_URL="${MANAGER_BASE:-http://localhost:18080}"
API_ENDPOINT="${MANAGER_URL}/v1/images/dockerhub/download"

# Default images to pre-load
IMAGES=(
  "nginx:latest"
  "postgres:15"
  "redis:7"
  "node:20"
  "python:3.11"
)

echo "🚀 Pre-loading default Docker images into registry..."
echo "Manager URL: ${MANAGER_URL}"
echo ""

# Function to download an image
download_image() {
  local image=$1
  echo "📦 Downloading ${image}..."

  response=$(curl -s -w "\n%{http_code}" -X POST "${API_ENDPOINT}" \
    -H "Content-Type: application/json" \
    -d "{\"image\":\"${image}\"}")

  http_code=$(echo "$response" | tail -n1)
  body=$(echo "$response" | sed '$d')

  if [ "$http_code" -eq 200 ]; then
    image_id=$(echo "$body" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
    echo "   ✅ Success! Image ID: ${image_id}"
  else
    echo "   ❌ Failed (HTTP ${http_code})"
    echo "   Response: ${body}"
  fi
  echo ""
}

# Download all images
for image in "${IMAGES[@]}"; do
  download_image "$image"
done

echo "🎉 Pre-load complete!"
echo ""
echo "View your images at: ${MANAGER_URL}/registry"
