# Holani-min

A command line frontend for the Atari Lynx emulator [Holani](https://github.com/LLeny/holani).

## Build

You will need [Rust and its package manager Cargo](https://www.rust-lang.org/). 

```
git clone https://github.com/LLeny/holani-min.git
```

Build with:

```
cargo build --release
```

The executable will be in the `target/release/` directory.

## Usage

> Run multiple instances to connect them through Comlynx.

```
Usage: holani-min [OPTIONS] --cartridge <CARTRIDGE>

Options:
  -c, --cartridge <CARTRIDGE>  Cartright, can be .o or a .lnx file
  -r, --rom <ROM>              ROM override
  -b, --buttons <BUTTONS>      Buttons mapping <up>,<down>,<left>,<right>,<out>,<in>,<o1>,<o2>,<pause> [default: up,down,left,right,q,w,1,2,p]
  -l, --linear                 Linear display filter
  -m, --mute                   Mute sound
  -x, --comlynx                Enable Comlynx
  -h, --help                   Print help
  -V, --version                Print version
```
