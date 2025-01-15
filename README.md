# Herbivore

Unofficial getgrass.io client

## Prerequisites

- Python 3.10+
- Poetry

## Installation

```bash
poetry install
```

## Usage

```bash
poetry run herbivore --userid <userid> --nodetype <nodetype>
```

## DietPi setup

```bash
sudo apt install python3-pip python3-venv build-essential
pip install poetry
poetry install
```

Run as service:

Do as root:

```bash
cd /root
git clone https://github.com/zippy1978/herbivore.git
nano /etc/systemd/system/herbivore.service
```

With content:

```bash
[Unit]
Description=Herbivore
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/poetry run herbivore --userid <userid> --nodetype <nodetype>
WorkingDirectory=/root/herbivore
Restart=always
User=root
Group=root
```

```bash
systemctl daemon-reload
systemctl start herbivore.service
```
