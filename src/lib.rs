// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 RemasteredArch
//
// This file is part of smtp_gateway.
//
// smtp_gateway is free software: you can redistribute it and/or modify it under the terms of the
// GNU Affero General Public License as published by the Free Software Foundation, either version
// 3 of the License, or (at your option) any later version.
//
// smtp_gateway is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See
// the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with
// smtp_gateway. If not, see <https://www.gnu.org/licenses/>.

//! # smtp_gateway
//!
//! smtp_gateway is a library for receiving SMTP messages.
//!
//! # How It Works
//!
//! [`crate::listen`] accepts any incoming TCP connection and spawns a new task to handle it as an
//! SMTP session. When an SMTP session finishes with a received message, it is passed to the
//! consumer to handle.
//!
//! smtp_gateway accepts messages but it cannot send or relay messages. An SMTP gateway receives
//! messages in SMTP and transform them for retransmission. smtp_gateway exists to handle the first
//! part of this goal, and it is up to the consumer to handle transformation and retransmission.
//!
//! For a real example of what this looks like, see smtp_gateway_bot. This is what smtp_gateway was
//! developed for, and can be found in the same repository as smtp_gateway:
//!
#![doc = concat!('<', env!("CARGO_PKG_REPOSITORY"), '>')]
//!
//! # Terminology
//!
//! smtp_gateway uses specific terminology (such as "client" and "server") as defined by [RFC 5321
//! section 2.3](https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3). Pull requests and issues
//! to fix discrepancies are welcome.

#![warn(clippy::nursery, clippy::pedantic)]
#![cfg_attr(debug_assertions, allow(clippy::missing_errors_doc))]

use std::io::Result;

use async_stream::try_stream;
use futures_core::stream::Stream;
use tokio::{net::TcpListener, task::JoinHandle};

mod connection;
mod message;
pub mod str;
#[cfg(test)]
mod test;
pub mod timeouts;
pub use message::Message;

pub type Session = JoinHandle<Result<()>>;

// This could return handles to tasks in an `async` iterator, where the consumer `await`s the next
// handle.
/// Listen on a port for incoming TCP connections and handle them as SMTP sessions.
///
/// # Errors
///
/// - [`std::io::Error`] from [`tokio::net::TcpListener::accept`].
/// - For I/O errors from a [`Session`], see [`connection::handle`].
pub fn listen(listener: TcpListener) -> impl Stream<Item = Result<Session>> {
    try_stream! {
        loop {
            let (stream, _) = listener.accept().await?;
            yield tokio::spawn(connection::handle(stream));
        }
    }
}

/// Tests whether a string is a domain name as considered by SMTP ([RFC 5321, section
/// 2.3.5](https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3.5)).
///
/// Checks whether any character is string is not alphanumeric, a dash (`'-'`), or a period
/// (`'.'`).
///
/// # Examples
///
/// ```rust
/// # use smtp_gateway::is_smtp_domain_name;
/// #
/// assert!(is_smtp_domain_name("example.com"));
/// assert!(is_smtp_domain_name("subdomain.example.com"));
/// assert!(is_smtp_domain_name("notld"));
/// assert!(!is_smtp_domain_name("example dot com"));
/// assert!(!is_smtp_domain_name("plus+.com"));
/// ```
#[must_use]
pub fn is_smtp_domain_name(str: &str) -> bool {
    !str.chars()
        .any(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
}

/// Read a line out of `reader`.
///
/// Returns a [`std::future::Future`], use with `.await`.
///
/// # Errors
///
/// - Any errors that could come out of the supplied reader's `read_line` function.
/// - If `read_line` reads zero bytes, [`std::io::ErrorKind::ConnectionAborted`] is returned.
#[macro_export]
macro_rules! read_line {
    ($reader:expr) => {
        async {
            let mut read_line_macro_buffer = String::new();
            match $reader.read_line(&mut read_line_macro_buffer).await {
                Ok(read_bytes) => {
                    if read_bytes == 0 {
                        Err(::std::io::ErrorKind::ConnectionAborted.into())
                    } else {
                        Ok(read_line_macro_buffer)
                    }
                }
                Err(e) => Err(e),
            }
        }
    };
}

/// Write a string literal into `writer` as an [`crate::str::SmtpString`]. Appends a line ending.
///
/// # Errors
///
/// - [`std::io::ErrorKind::InvalidInput`] if passed invalid ASCII.
/// - Any errors that could come out of the supplied writer's `write_all` function.
#[macro_export]
macro_rules! write_line {
    ($writer:expr, $str:expr) => {{
        match $crate::str::SmtpString::new(concat!($str, "\r\n")) {
            Ok(s) => $writer.write_all(s.as_bytes()).await,
            Err(e) => Err(::std::io::Error::new(::std::io::ErrorKind::InvalidInput, e)),
        }
    }};
}

/// Write a format statement into `writer` as an [`crate::str::SmtpString`]. Appends a line ending.
///
/// All but the first parameter are passed directly into [`format`].
///
/// # Errors
///
/// - [`std::io::ErrorKind::InvalidInput`] if passed invalid ASCII.
/// - Any errors that could come out of the supplied writer's `write_all` function.
#[macro_export]
macro_rules! write_fmt_line {
    ($writer:expr, $( $fmt:expr ),+) => {{
        match $crate::str::SmtpString::new(&format!("{}\r\n", format!( $($fmt),+ ))) {
            Ok(s) => $writer.write_all(s.as_bytes()).await,
            Err(e) => Err(::std::io::Error::new(::std::io::ErrorKind::InvalidInput, e)),
        }
    }};
}
