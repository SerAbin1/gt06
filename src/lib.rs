//! Parser and stream decoder for the GT06 GPS tracker protocol.
//!
//! GT06 is used by a large family of low-cost GPS trackers to report
//! location, status and alarm events over a raw TCP connection.
//!
//! Use [`Decoder`] to reassemble [`Message`]s out of a raw byte stream —
//! it buffers partial reads, so it's safe to feed it directly from a
//! socket. Use [`parse_packet`] instead if you already have one complete,
//! framed packet.
//!
//! Login and status messages expect an acknowledgement written back to the
//! device; build one with [`build_ack`].
//!
//! ```
//! use gt06::{build_ack, Decoder, Message};
//!
//! # let raw_bytes_from_socket: [u8; 18] = [
//! #     0x78, 0x78, 0x0d, 0x01, 0x03, 0x56, 0x93, 0x80, 0x35, 0x64, 0x38, 0x09,
//! #     0x00, 0x01, 0x91, 0x1f, 0x0d, 0x0a,
//! # ];
//! let mut decoder = Decoder::new();
//! for result in decoder.push(&raw_bytes_from_socket) {
//!     match result {
//!         Ok(Message::Login(login)) => {
//!             println!("device {} connected", login.imei);
//!             let ack = build_ack(0x01, login.serial_number);
//!             // socket.write_all(&ack)?;
//!             assert_eq!(ack[3], 0x01);
//!         }
//!         Ok(other) => println!("{other:?}"),
//!         Err(err) => eprintln!("bad packet: {err}"),
//!     }
//! }
//! ```

mod crc;
mod datetime;
mod error;
mod frame;
mod message;
mod parse;
mod response;

pub use error::Error;
pub use frame::Decoder;
pub use message::*;
pub use parse::parse_packet;
pub use response::build_ack;
