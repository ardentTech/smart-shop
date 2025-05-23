# Adafruit Feather RP2040 RFM95 Quickstart
This repo contains a barebones template for writing Rust firmware for the [Adafruit Feather RP2040 RFM95 board](https://www.adafruit.com/product/5714).

## Features
* [Embassy](https://embassy.dev/) (embrace the async)
* Board Support Package (BSP): handle pin mapping and grouping in a single location
* USB logging: the board doesn't expose the SWD pins on the RP2040

## Initial Setup
1. `$ cargo generate --git https://github.com/ardentTech/adafruit-feather-rp2040-rfm95-quickstart.git`
2. Set up a serial port communication program on your host (e.g. [minicom](https://github.com/Distrotech/minicom))

## Commands
* Build: `$ cargo build --release`
* Flash: `$ cargo run --release`

## Boot2
The board uses an external Winbond W25Q64JV Flash chip, and since this chip can and does vary amongst different
boards, a second stage bootloader is required to configure the external Flash memory. While `embassy-rp` is a dependency
and the `rp2040` feature is brought into scope, what is NOT scoped is a related boot2 implementation. This is because
the W25Q64JV is not explicitly supported within `embassy-rp`. With no explicit boot2, `embassy-rp` defaults to the
W25Q080 boot2 implementation, which **appears to just work** with the Adafruit Feather RP2040 RFM95 board. Neat.

If you want to want more info about the RP2040's boot sequence, check out this great article from [Van Hunter Adams](https://vanhunteradams.com/Pico/Bootloader/Boot_sequence.html).