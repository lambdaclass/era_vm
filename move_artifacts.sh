#!/usr/bin/env bash

## This script exists because for some reason currently the `zksolc` compiler
## puts the resulting compilation artifacts in different directories on linux
## vs macos. On macos, it puts them where you specify it (i.e. on the directory
## passed through the --output argument). On Linux, it puts them the binary
## and assembly files under the `/programs` directory inside the output directory
## specified by the user. The CI workflow uses this script to move the artifacts
## so that file paths on integration tests don't have to be modified depending
## on the underlying operating system.

set -e

for dir in ./program_artifacts/*/
do
    mv ${dir}/programs/* ${dir}/
done
