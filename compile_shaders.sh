#!/bin/bash

cd evn_shaderc
cargo build --release
cd ..

./evn_shaderc/target/release/evn_shaderc -i ./src/shaders/ -o ./resources/closed/shaders/
