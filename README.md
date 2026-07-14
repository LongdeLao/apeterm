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
unofficial `pytr` project. Phone, PIN, and verification code are entered inside
the Portfolio TUI login modal; ApeTerm only keeps them in memory for that login
step, while pytr owns credentials and session cookies under `~/.pytr`.

Install ApeTerm with optional broker dependencies:

```bash
curl -fsSL https://github.com/LongdeLao/apeterm/raw/master/install.sh | INSTALL_BROKER_DEPS=1 bash
```

If you are working from a checkout instead:

```bash
INSTALL_BROKER_DEPS=1 BUILD_FROM_SOURCE=1 ./install.sh
```

Open ApeTerm, go to Portfolio from `Ctrl+P`, then use the broker controls:

- `c` connects Trade Republic
- `r` syncs the read-only portfolio snapshot
- `d` disconnects Trade Republic from ApeTerm

Connect opens ApeTerm's TUI login modal. Enter your phone number, PIN, and the
Trade Republic code/TAN when requested. ApeTerm stores only the normalized
portfolio snapshot in its own application data directory.

Because pytr uses Trade Republic's private API, login or sync can temporarily
break when Trade Republic changes it. Press `c` in Portfolio again when a
session expires.
