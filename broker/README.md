# Optional broker adapters

Broker support is opt-in and read-only. Trade Republic is implemented through
[`pytr`](https://github.com/pytr-org/pytr), an unofficial client for Trade
Republic's private API. It is not affiliated with Trade Republic Bank GmbH.

## Trade Republic

Install the optional dependency:

```bash
python -m pip install -r broker/requirements.txt
```

Open ApeTerm, use `Ctrl+P` to open Portfolio, then use the broker controls:

- `c` connects Trade Republic
- `r` syncs the read-only portfolio snapshot
- `d` disconnects Trade Republic from ApeTerm

Connect runs pytr's interactive web login and lets pytr own credentials and
session cookies in `~/.pytr`. ApeTerm only stores a normalized portfolio JSON
snapshot in its application data directory. Syncing never places orders.

Because pytr uses a private API, Trade Republic changes can temporarily break
login or sync. Press `c` in Portfolio again when a web session expires.
