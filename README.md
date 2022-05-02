
[![Rust CI](https://github.com/StripedMonkey/ws_relay_server/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/StripedMonkey/ws_relay_server/actions/workflows/rust-ci.yml)
### Building

* Follow the instructions on https://www.rust-lang.org/tools/install
* run `cargo build --release` in the project directory
* Output will be `target/release/signaling_server`

Note: For those using a newer version of Libc you may need to build a musl version without libc in order to run on machines with older versions of libc (I.E build on arch linux, deploying to Ubuntu 20). To do so you install the musl target and run `cargo build --release --target <Target>` (in the case of linux `x86_64-unknown-linux-musl`)

### Getting into rooms

* Creating a room
  * Send `"NewRoom"` as the first message when connecting
  * The WebSocket will respond with the following:
    * `{"JoinRoom":"<ROOM-ID>"}`
* Joining an already existing room
  * Send `{"JoinRoom":"<ROOM-ID>"}` as the first message when connecting
* Creating a room with a specific ID
  * Send `{"JoinWithCode" "<ROOM-ID>"}`




After rooms have been joined and created any messages sent by any client will be seen by all other clients in the room
