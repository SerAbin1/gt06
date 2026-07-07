# gt06 (WIP)

This is a port of my JS-based gt06 parser to Rust, with significant AI use. It's more a
learning project in my Rust journey than it is a production-ready library — though the
original JS-based parser has been running in prod for a while now.

I'll be continuing work on this library as time allows, and will mark `1.0.0` when I feel
it's ready for general use.

## What it does

Parses the GT06 GPS tracker protocol: login, location (standard and extended), status,
and alarm messages, sent over a raw TCP connection.

- [`Decoder`] reassembles messages out of a raw byte stream, buffering partial reads.
- [`parse_packet`] parses a single, already-framed packet.
- [`Message::ack_bytes`] builds the acknowledgement packet required for login/status
  messages, or `None` for message types that don't need one.

## Example

```rust
use gt06::{Decoder, Message};

let mut decoder = Decoder::new();

for result in decoder.push(&bytes_from_socket) {
    match result {
        Ok(message) => {
            if let Message::Login(login) = &message {
                println!("device {} connected", login.imei);
            }
            if let Some(ack) = message.ack_bytes() {
                socket.write_all(&ack)?;
            }
        }
        Err(err) => eprintln!("bad packet: {err}"),
    }
}
```

## License

MIT
