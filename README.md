# Steam TUI
[![tests](https://github.com/dmadisetti/steam-tui/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/dmadisetti/steam-tui/actions/workflows/test.yml) [![Crates.io](https://img.shields.io/crates/v/steam-tui.svg)](https://crates.io/crates/steam-tui)

## About
Just a simple TUI client for `steamcmd`. Allows for the graphical launching,
updating, and downloading of steam games through a simple terminal client.
Methodology informed by [steam-cli](https://github.com/berenm/steam-cli).

<p align="center">
  <img width="600" alt="Example of rules_euler in action" src="screenshot.png">
</p>

## Usage

Login with `steamcmd` first to cache your credentials (don't trust some random app with your passwords):
```bash
steamcmd
# Steam> login <user>
# Steam> quit
```
Launch the binary `steam-tui`, and rejoice :tada:. Help is in the client.

Unable to launch games? Pressing space will start a bare-bones steam client (no
graphics) in the background and will let you launch games that need steam
libraries or have some sort of DRM.

## Features not in the help

It's like an Easter egg for reading documentation!

### Favourites
Pressing `f` will toggle favourites on a game, pressing `F` will filter favourite games.

### Hiding games
Pressing `H` will hide the selected game. Hidden games are recorded in `~/.config/steam-tui/config.json`.

### Showing other things (like demos)
You can enable (or hide by exclusion) `Game` `DLC` `Driver` `Applications` `Config` `Demo` `Tool` `Unknown`, by changing the `allowed_games` field in the config.

## Requirements

[`steamcmd`](https://wiki.archlinux.org/title/steam#SteamCMD) is required to
launch steam-tui, as `steam-tui` is essentially just a graphical wrapper for
this program. `wine` usage will be attempted if a native Linux game is not
found.

## Why ?
~Because why not? Also, the Steam client seems to break on my Arch build. I have
a GT 610, and barely anything graphical works- this is a nice work around.~

**Update**, I got rid of the 610 (let's go 1660), but moved to NixOS and the steam
client still doesn't work lol.

## Contributing

At this point, I am very much done with the project. I only really play
[Kerbal](https://www.kerbalspaceprogram.com/) and [5 Dimensional Chess with
Multiverse Time Travel](https://www.5dchesswithmultiversetimetravel.com/), so
additional work on this project is moot. If you [buy me a
coffee](https://github.com/sponsors/dmadisetti), I'd be happy to sink some more
time into this.

## Missing Features

- Better handling for Proton games
