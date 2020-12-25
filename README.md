# Rust CHIP-8 Emulator
This is a [CHIP-8](https://wikipedia.org/wiki/CHIP-8) emulator written in Rust and compiled to WebAssembly. You can try it [here](https://galhorowitz.github.io/WASM-CHIP8Emulator/).

## Usage
1. Select and load a ROM from the list of built-in ROMs, or upload a ROM from your computer.
2. Click `Start Game`
3. Either use the on-screen keyboard, or use the QWERTY keyboard mapping available when hovering over `Hex Keyboard(?)`
4. <i>(Optional:)</i> If the ROM is not functioning correctly and it was written for the original CHIP-8 interpreter, try changing the options under `Advanced Settings`

## Project Structure
The actual CHIP-8 CPU emulation is written entirely in Rust, at `src/lib.rs`, and is a port of my C++ CHIP-8 Emulator that I wrote to practice Rust.
I also wrote some JS glue code (`static/index.js`) to handle the interface and canvas/audio, but I am not a web developer, so it might not be quality JS code.

## Compiling
To compile you'll need the standard Rust toolchain (rustup, rustc, cargo).
You also need `wasm-pack` which automatically generates JS bindings for exported Rust functions.
To compile run:
```bash
wasm-pack build --target=web --no-typescript --out-dir=static/pkg --release
```

## ROMs
This repository contains ROMs from [badlogic's repo](https://github.com/badlogic/chip8/tree/master/roms) that can be selected in the website.