# evn

A hobby game with a selfmade engine written in Rust

## Resource System

- Resource folder (`resources/`)
  - Closed (`resources/closed/`)
    - Included in the executable with the include_resource! macro
    - Shouldn't change (Textures, Shaders, ...)
  - Open (`resources/open/`)
    - Loaded at runtime
    - Meant to change (Configs, ...)
    - In the executables directory when packed

## Shader compilation

Done with evn_shaderc, a command line wrapper around [shaderc](https://github.com/google/shaderc-rs).

- ### Usage

`./compile_shaders.sh`

#### This will

- Compile evn_shaderc in release mode
- Run evn_shaderc with the correct arguments
  - Compiles GLSL shaders in `src/shaders/`
  - Saves them in `resources/`

#### Requirements

- Linux (maybe provide bat script for windows?)

## Building / Running

- ### Development Mode

`cargo run --release -- --dev`

- ### Packed Mode

`./pack_game.sh [-l | --linux] [-w | --windows]`

#### This will

- Compile evn in release mode
- Strip the executable of symbols (size reduction)
- Generate a zip with the executable and open resources

#### Requirements

- Linux (maybe provide bat script for windows?)
- A [windows cross compiler](https://github.com/japaric/rust-cross/blob/master/README.md) when compiling to windows
