# Herbivore

Unofficial Rust client for getgrass.io

## Prerequisites

- Rust 1.70+

## Installation

```bash
git clone https://github.com/zippy1978/herbivore.git
cd herbivore
cargo install --path .
```

## Usage

```bash
Usage: herbivore [OPTIONS] --user-id <USER_ID>

Options:
  -u, --user-id <USER_ID>      
  -n, --node-type <NODE_TYPE>  [default: 1.25x] [possible values: 1x, 2x, 1.25x]
  -h, --help                   Print help
  -V, --version                Print version
```

## DietPi setup

```bash
sudo apt install build-essential libssl-dev pkg-config
curl https://sh.rustup.rs -sSf | sh
cd /root
git clone https://github.com/zippy1978/herbivore.git
cd herbivore
cargo install --path .
```

Run as service:

Do as root:

```bash
nano /etc/systemd/system/herbivore.service
```

With content:

```bash
[Unit]
Description=Herbivore
After=network.target

[Service]
Type=simple
ExecStart=/root/.cargo/bin/herbivore --user-id <userid> --node-type <nodetype> --log-file /var/log/herbivore.log
WorkingDirectory=/root/herbivore
Restart=always
User=root
Group=root
```

```bash
systemctl daemon-reload
systemctl start herbivore.service
```
