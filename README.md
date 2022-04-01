Gateman
===

Flow control with

### Bi-directional connection that supports
1. Ensuring that the client device is connected
2. Receiving commands from the client device
3. Sending status updates to the client device

### Fail-safes that must be in place
1. Shutting the sytem down in a controlled manner if client is non-responsive or disconnects


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

Boot config (`/boot/config.txt`) needs an entry to configure GPIO (BCM) 12 as PWM0

`dtoverlay=pwm,pin=12,func=4`

## License

GPL v3
