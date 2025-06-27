## Building
1. Press `boot` button on target board and attach to host via USB (without OLED feather)
2. `$ cd env_sensor && cargo build --release`
3. Attach OLED feather and press `reset` button on feather

## Testing
* `$ cargo test --package sht30`
* * `$ cargo test --package air_quality`