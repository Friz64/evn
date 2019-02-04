# evn

A hobby game with a selfmade engine written in Rust

## Screenshots

| The first triangle |
| - |
| ![The first triangle!](https://cdn.discordapp.com/attachments/342294639363227648/541792444279291905/unknown.png) |


## Resource System

System for loading external files asynchronously.

- Resource folder (`resources/`)
  - Closed (`resources/closed/`)
    - Shouldn't change (Textures, Shaders, ...)
    - Packed path: `./res/`
  - Open (`resources/open/`)
    - Meant to change (Configs, ...)
    - Packed path: `./`

## Building / Running

```
USAGE:
    evn [FLAGS]

FLAGS:
        --debug-callback    Enable Vulkan debug callback
        --dev               Enable Development mode
    -h, --help              Prints help information
    -c, --no-color          Don't color the console log
    -V, --version           Prints version information
```

- ### Development Mode

`cargo run -p evn --release -- --dev`

- ### Packed Mode

`./pack_game.sh [-l | --linux] [-w | --windows]`

#### This will

- Compile evn in release mode
- Strip the executable of symbols (size reduction)
- Generate a zip with the executable and open resources

#### Requirements

- Linux (maybe provide bat script for windows?)
- A [windows cross compiler](https://github.com/japaric/rust-cross/blob/master/README.md) when compiling to windows

## Shader compilation

Done with evn_shaderc, a command line wrapper around [shaderc](https://github.com/google/shaderc-rs).

- ### Usage

`./compile_shaders.sh`

#### This will

- Compile evn_shaderc in release mode
- Run evn_shaderc with the correct arguments
  - Compiles GLSL shaders in `evn/src/shaders/`
  - Saves them in `resources/closed/shaders/`

#### Requirements

- Linux (maybe provide bat script for windows?)
