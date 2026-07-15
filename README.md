# gt06

[![crates.io](https://img.shields.io/crates/v/gt06.svg)](https://crates.io/crates/gt06)
[![docs.rs](https://img.shields.io/docsrs/gt06)](https://docs.rs/gt06)
[![license](https://img.shields.io/crates/l/gt06.svg)](./LICENSE)

Parser and stream decoder for the GT06 GPS tracker protocol, with **zero dependencies**.

GT06 is used by a large family of low-cost GPS trackers to report location,
status, and alarm events to a server over a raw TCP connection. This crate turns
that byte stream into typed Rust values and builds the acknowledgement packets
the devices expect back.

> **Status:** pre-1.0. The API may change between minor versions until `1.0.0`.

## Features

- **Stream decoder** that reassembles packets from arbitrarily-chunked TCP reads,
  buffering partial packets.
- **Single-packet parser** for already-framed packets if you need it.
- **Acknowledgement builder** for the message types that require a reply.
- Typed messages and enums for every supported field.

## Supported messages

| Protocol | Message            | Parsed as           | Needs ACK |
| -------- | ------------------ | ------------------- | --------- |
| `0x01`   | Login              | `Message::Login`    | yes       |
| `0x12`   | Location           | `Message::Location` | no        |
| `0x22`   | Extended location  | `Message::Location` | no        |
| `0x13`   | Status / heartbeat | `Message::Status`   | yes       |
| `0x16`   | Alarm              | `Message::Alarm`    | no        |

## Installation

```sh
cargo add gt06
```

Or add it to `Cargo.toml`:

```toml
[dependencies]
gt06 = "0.2"
```

## Quick start

Feed raw bytes into a `Decoder`, handle each `Message`, and write back any ACK it
asks for:

```rust
use gt06::{Decoder, Message};

let mut decoder = Decoder::new();

// `chunk` is whatever you just read off the socket.
for result in decoder.push(&chunk) {
    match result {
        Ok(message) => {
            if let Message::Login(login) = &message {
                println!("device {} connected", login.imei);
            }
            if let Some(ack) = message.ack_bytes() {
                // socket.write_all(&ack)?;
            }
        }
        Err(err) => eprintln!("bad packet: {err}"),
    }
}
```

`decoder.push()` returns one result per complete packet found. An incomplete
trailing packet is kept in the decoder's buffer and completed on a later `push()`,
so it's safe to feed it directly from a socket in whatever sizes reads arrive.

## Usage

### With a blocking `std` TCP server

```rust
use std::io::{Read, Write};
use std::net::TcpListener;
use gt06::Decoder;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:5023")?;

    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut decoder = Decoder::new();
        let mut buf = [0u8; 1024];

        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                break; // connection closed
            }

            for result in decoder.push(&buf[..n]) {
                match result {
                    Ok(message) => {
                        if let Some(ack) = message.ack_bytes() {
                            stream.write_all(&ack)?;
                        }
                    }
                    Err(err) => eprintln!("bad packet: {err}"),
                }
            }
        }
    }
    Ok(())
}
```

### With an async `tokio` TCP server

`gt06` has no async code of its own — the `Decoder` is a plain sync state
machine, so you just drive it from your own read loop.

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use gt06::Decoder;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:5023").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut decoder = Decoder::new();
            let mut buf = [0u8; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => break,          // connection closed
                    Ok(n) => n,
                    Err(_) => break,
                };

                for result in decoder.push(&buf[..n]) {
                    match result {
                        Ok(message) => {
                            if let Some(ack) = message.ack_bytes() {
                                let _ = socket.write_all(&ack).await;
                            }
                        }
                        Err(err) => eprintln!("bad packet: {err}"),
                    }
                }
            }
        });
    }
}
```

### Parsing a single, already-framed packet

If you already have one complete packet (start marker through end marker) — say,
from a log file or a test — use `parse_packet`. It carries no connection state,
so the `imei` field on location/status/alarm messages is always `None`.

```rust
use gt06::{parse_packet, Message};

let packet: [u8; 18] = [
    0x78, 0x78, 0x0d, 0x01, 0x03, 0x56, 0x93, 0x80, 0x35, 0x64, 0x38, 0x09,
    0x00, 0x01, 0x91, 0x1f, 0x0d, 0x0a,
];

match parse_packet(&packet) {
    Ok(Message::Login(login)) => println!("IMEI: {}", login.imei),
    Ok(other) => println!("{other:?}"),
    Err(err) => eprintln!("parse error: {err}"),
}
```

### Reading message data

```rust
use gt06::Message;

match message {
    Message::Login(login) => {
        println!("{}", login.imei);
    }
    Message::Location(loc) => {
        // `loc.imei` is Some(..) once a login was seen on the connection.
        println!("{}, {}", loc.fix.latitude, loc.fix.longitude);
        println!("{} km/h, heading {}", loc.fix.speed_kmh, loc.fix.course);
        println!("unix time (UTC): {}", loc.fix.time);
    }
    Message::Status(status) => {
        println!("ignition on: {}", status.flags.ignition);
        println!("battery: {:?}", status.voltage_level);
        println!("signal: {:?}", status.gsm_signal);
    }
    Message::Alarm(alarm) => {
        println!("alarm: {:?}", alarm.alarm);
        println!("at {}, {}", alarm.fix.latitude, alarm.fix.longitude);
    }
}
```

### Building an ACK by hand

`Message::ack_bytes()` covers the common case, but you can also build one
directly from a protocol byte and serial number:

```rust
use gt06::build_ack;

let ack = build_ack(0x01, 1); // login ack for serial number 1
// -> [0x78, 0x78, 0x05, 0x01, 0x00, 0x01, 0xd9, 0xdc, 0x0d, 0x0a]
```

## Handling errors

`decoder.push()` returns a `Result` per packet, and a bad packet never stops the
stream — the decoder resynchronizes on the next start marker and keeps going. The
`Error` enum covers short buffers, missing start/end markers, CRC mismatches,
unknown protocol numbers, and malformed messages. All variants implement
`Display` and `std::error::Error`.

## Notes & caveats

- **Timestamps** are Unix seconds in UTC (`Fix::time`).
- **Coordinates** are signed decimal degrees, rounded to 6 decimal places.
- Several `Fix` fields (satellites, cell tower info) are `None` for extended
  (`0x22`) location packets, which don't carry them.
- The extended (`0x22`) layout follows a length-based heuristic mirroring the
  reference implementation, as there's no single authoritative published layout.

## API reference

Full documentation for every type, method, field, and enum variant lives on
[docs.rs/gt06](https://docs.rs/gt06). The main entry points are:

- `Decoder` — `new()`, `push()`, `imei()`
- `parse_packet()` — parse a single framed packet
- `build_ack()` — build an acknowledgement packet
- `Message` — `ack_bytes()`, `expects_ack()`

## Testing

```sh
cargo test
```

## Contributing

Issues and pull requests are welcome.

## License

MIT — see [LICENSE](./LICENSE).
