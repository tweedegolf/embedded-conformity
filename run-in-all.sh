#!/bin/bash

# shift

for dir in esp32c6 nRF52 runner shield stm32 test-suite
do
    pushd $dir > /dev/null
    echo "--- Running in $dir"
    $@
    popd > /dev/null
done
