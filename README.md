Gateman
===

Flow control

## Setup

### Cross compile

```
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf
```

Add the following entry to the Cargo config toml

```
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

(this is now included in the repo)

https://doc.rust-lang.org/cargo/reference/config.html

### Pi PWM

Boot config (`/boot/config.txt`) needs an entry to configure GPIO (BCM) 12 as PWM0

`dtoverlay=pwm,pin=12,func=4`


## Build

```
cargo build --target=armv7-unknown-linux-gnueabihf
```

## License

GPL v3
