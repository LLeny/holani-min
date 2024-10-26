# Holani-min
A minimal frontend for the Atari Lynx emulator [Holani](https://github.com/LLeny/holani).

## Build
You will need [Rust and its package manager Cargo](https://www.rust-lang.org/). 

```
git clone https://github.com/LLeny/holani-min.git
```

Build the debugger with:

```
cargo build --release
```

The executable will be in the `target/release/` directory.

## Usage

```
Usage: holani-min [OPTIONS] --cartridge <CARTRIDGE>

Options:
  -c, --cartridge <CARTRIDGE>  Cartright, can be .o or a .lnx file
  -r, --rom <ROM>              ROM override
  -l, --linear                 Linear display filter
  -m, --mute                   Mute sound
  -b, --buttons <BUTTONS>      Buttons mapping <up><down><left><right><outside><inside><option_1><option_2)>, Default ikjlqw12 [default: ikjlqw12]
  -h, --help                   Print help
  -V, --version                Print version
```
