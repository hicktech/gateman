Gateman
===

Flow control

## Setup

### Rust cross compile

#### Setup

```
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf
```

#### Cargo
Add the following entry to `~/.cargo/config.toml`

``
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
``

https://doc.rust-lang.org/cargo/reference/config.html

#### Build

```
cargo build --target=armv7-unknown-linux-gnueabihf
```


### Pi PWM config

Boot config needs an entry to configure GPIO (BCM) 12 as PWM0

`dtoverlay=pwm,pin=12,func=4`

## License

GPL v3
