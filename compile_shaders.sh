#!/bin/bash

cargo build -p evn_shaderc --release

./target/release/evn_shaderc -i ./evn/src/shaders/ -o ./resources/closed/shaders/
