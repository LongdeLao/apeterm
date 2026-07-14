# ApeTerm

## Install

1. Do this:

```bash
curl -fsSL https://github.com/LongdeLao/apeterm/raw/master/install.sh | bash
```
2. Run this:

```bash
apeterm
```


If it's not working then open a new terminal and run it again.

What this script does:
- puts `apeterm` on your path
- installs the app runtime under `~/.local/share/apeterm`
- sets up the private Python runtime ApeTerm uses for `yfinance`

## New workspaces

Press `Ctrl+P` to open the command palette. Portfolio, Alerts, Screener,
Compare, Calendar, and four saved dashboard presets are available as pages or
dashboard panels. Narrow terminals automatically switch to a focused pane.

## Add Trade Republic portfolio

Trade Republic support is read-only and disabled by default. It uses the
unofficial `pytr` project; ApeTerm never reads or stores the phone number, PIN,
verification code, or pytr session cookies.

Install ApeTerm with optional broker dependencies:

```bash
curl -fsSL https://github.com/LongdeLao/apeterm/raw/master/install.sh | INSTALL_BROKER_DEPS=1 bash
```

If you are working from a checkout instead:

```bash
INSTALL_BROKER_DEPS=1 BUILD_FROM_SOURCE=1 ./install.sh
```

Connect Trade Republic and import the read-only portfolio snapshot:

```bash
apeterm broker connect
apeterm broker sync
apeterm broker status
```

`connect` starts pytr's interactive login. pytr owns credentials and session
cookies under `~/.pytr`; ApeTerm only stores the normalized portfolio snapshot
in its own application data directory.

Use this to remove Trade Republic data from ApeTerm:

```bash
apeterm broker disconnect
```

Because pytr uses Trade Republic's private API, login or sync can temporarily
break when Trade Republic changes it. Re-run `apeterm broker connect` when a
session expires.
