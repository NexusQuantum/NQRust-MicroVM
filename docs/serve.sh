#!/bin/bash
# Hugo development server script

export PATH=/home/shiro/go-binary/bin:$PATH
export GOPATH=/home/shiro/go

cd "$(dirname "$0")"
../bin/hugo server --buildDrafts --bind 0.0.0.0 --port 1313
