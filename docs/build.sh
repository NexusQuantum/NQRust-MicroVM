#!/bin/bash
# Hugo build script

export PATH=/tmp/go/bin:$PATH
export GOPATH=/home/shiro/go

cd "$(dirname "$0")"
../bin/hugo --minify
