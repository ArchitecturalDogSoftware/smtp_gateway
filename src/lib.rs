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

/// Write a string literal into `writer` as an [`str::SmtpString`]. Appends a line ending.
///
/// # Errors
///
/// - Any errors that could come out of the supplied writer's `write_all` function.
///
/// # Panics
///
/// Panics (at compile time) if passed invalid ASCII.
#[macro_export]
macro_rules! write_line {
    ($writer:expr, $str:expr) => {{
        const STR: $crate::str::RawSmtpStr<{ $crate::str::max_lengths::REPLY_LINE }> =
            $crate::str::RawSmtpStr::new(concat!($str, "\r\n"));
        $writer.write_all(STR.as_bytes()).await
    }};
}

/// Write a format statement into `writer` as an [`crate::str::SmtpString`]. Appends a line ending.
///
/// All but the first parameter are passed directly into [`format`].
///
/// # Errors
///
/// - [`std::io::ErrorKind::InvalidInput`] if the string contains invalid ASCII after
///   formatting.
/// - Any errors that could come out of the supplied writer's `write_all` function.
///
/// # Panics
///
/// Panics (at compile time) if the format string contains invalid ASCII. Use `"{}", variable`
/// syntax if `variable` needs to be named with non-ASCII characters, as neither succeeded inputs
/// or the resulting output are checked at compile time.
///
/// # Examples
///
/// ```
/// use tokio::io::AsyncWriteExt;
/// use smtp_gateway::write_fmt_line;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut writer = tokio_test::io::Builder::new().write(b"formatted string\r\n").build();
///
/// write_fmt_line!(writer, "formatted {}", "string")?;
/// #     Ok(())
/// # }
/// ```
///
/// Non-ASCII in the formatting string will be caught at compile time:
///
/// ```compile_fail
/// # use tokio::io::AsyncWriteExt;
/// # use smtp_gateway::write_fmt_line;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     let mut writer = tokio_test::io::Builder::new().write("fmt 🦀\r\n".as_bytes()).build();
/// // Panics at compile time due to invalid ASCII.
/// write_fmt_line!(writer, "fmt 🦀")?;
/// #     Ok(())
/// # }
/// ```
///
/// Non-ASCII in the resulting string will be caught at runtime:
///
/// ```should_panic
/// # use tokio::io::AsyncWriteExt;
/// # use smtp_gateway::write_fmt_line;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     let mut writer = tokio_test::io::Builder::new().write("fmt 🦀\r\n".as_bytes()).build();
/// // Errors at runtime due to invalid ASCII.
/// write_fmt_line!(writer, "fmt {}", '🦀')?;
/// #     Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! write_fmt_line {
    ($writer:expr, $fmt_str:expr $(, $fmt_item:expr )*) => {{
        // Causes a compile time panic if `$fmt_str` contains non-ASCII characters.
        const _: () = {
            assert!($fmt_str.is_ascii(), "invalid ASCII in format string");
        };

        match $crate::str::SmtpString::new(&format!("{}\r\n", format!( $fmt_str, $($fmt_item),* ))) {
            Ok(s) => $writer.write_all(s.as_bytes()).await,
            // Runtime error that occurs if the formatted output contains non-ASCII characters.
            Err(e) => Err(::std::io::Error::new(::std::io::ErrorKind::InvalidInput, e)),
        }
    }};
}
