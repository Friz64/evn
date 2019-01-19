#!/bin/bash

# https://stackoverflow.com/a/33826763
while [[ "$#" > 0 ]]; do case $1 in
    -l|--linux) linux=1;;
    -w|--windows) windows=1;;
    *) echo "Unknown parameter passed: $1"; exit 1;;
esac; shift; done

if [ "$linux" != "1" ] && [ "$windows" != "1" ]; then
    echo "No build options specified"
fi

if [ "$linux" == "1" ]; then
    echo "BUILDING LINUX"
    cargo build --release
    strip target/release/evn

    echo ""
    echo "PACKING LINUX"
    cp -r resources/open/ temp/
    cp target/release/evn temp/
    cd temp
    zip -r evn_linux.zip .
    cd ..
    mv temp/evn_linux.zip .
    rm -r temp/

    echo ""
fi

if [ "$windows" == "1" ]; then
    echo "BUILDING WINDOWS"
    cargo build --release --target x86_64-pc-windows-gnu
    strip target/x86_64-pc-windows-gnu/release/evn.exe

    echo ""
    echo "PACKING WINDOWS"
    cp -r resources/open/ temp/
    cp target/x86_64-pc-windows-gnu/release/evn.exe temp/
    cd temp
    zip -r evn_windows.zip .
    cd ..
    mv temp/evn_windows.zip .
    rm -r temp/

    echo ""
fi
