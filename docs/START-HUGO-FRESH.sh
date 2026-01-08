#!/bin/bash
# Script to start Hugo with fresh build (no cache)

echo "🧹 Cleaning cache and build folders..."
cd "$(dirname "$0")"

# Remove cache and build folders
rm -rf public/ resources/

echo "✅ Cache cleared"
echo ""
echo "🚀 Starting Hugo server..."
echo "📍 Access at: http://localhost:1313/docs/"
echo ""

export PATH=/home/shiro/go-binary/bin:$PATH
export GOPATH=/home/shiro/go

# Start with fresh build
../bin/hugo server --buildDrafts --bind 0.0.0.0 --port 1313 --disableFastRender
