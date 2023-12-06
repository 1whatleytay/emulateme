# Emulateme - NES Emulator

Emulateme is an NES emulator written in Rust. It has a GUI interface `emgui`, and a server interface for connections from other clients `emserver`.

## Running `emgui` or `emserver`

To run emserver, you have to have Rust installed (`cargo`).
You must also have a ROM of the game you want to play.

Once Rust is installed, simply:
```shell
# Enter the emserver source directory.
cd emserver # OR cd emgui

# Build and run the binary.
cargo run /path/to/game.nes
```

The server will be hosted on port `9013`.
