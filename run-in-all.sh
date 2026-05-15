#!/bin/bash

status=0

for dir in esp32c6 nRF52 runner shield stm32 test-suite
do
    pushd $dir > /dev/null
    echo "--- Running in $dir" 1>&2
    $@

    if $@
    then
        echo "Success" 1>&2
    else
        echo "Failed" 1>&2
        status=1
    fi

    popd > /dev/null
done

exit $status
