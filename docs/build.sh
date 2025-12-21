#!/bin/bash
# Hugo build script

export PATH=/home/shiro/go-binary/bin:$PATH
export GOPATH=/home/shiro/go

cd "$(dirname "$0")"
../bin/hugo --minify
