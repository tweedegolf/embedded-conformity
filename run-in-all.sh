#!/bin/bash

status=0

for dir in esp32c6 nRF52 runner shield stm32 test-suite
do
    pushd $dir > /dev/null
    echo "--- Running in $dir"
    $@

    if $@
    then
        echo "Success"
    else
        echo "Failed"
        status=1
    fi

    popd > /dev/null
done

exit $status
