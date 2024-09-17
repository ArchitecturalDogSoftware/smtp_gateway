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

//! Handles TCP connections as SMTP sessions.
//!
//! See [`handle`].

mod command;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::error::Elapsed,
};

use crate::write_fmt_line;

const DOMAIN: &str = "example.com";

/// Handle a TCP connection as an SMTP session.
///
/// # Errors
///
/// This function will return [`std::io::Error`] from a variety of sources:
///
/// - I/O errors from [`AsyncWriteExt::write_all`] on [`TcpStream`].
/// - I/O and UTF-8 errors from [`AsyncBufReadExt::read_line`] on [`BufReader<TcpStream>`].
/// - I/O errors encountered in [`TcpStream::local_addr`] amd [`TcpStream::peer_addr`].
///     - On POSIX, these come from `getsockname` and `getpeername` from the C standard library.
///       If these return explicit errors or malformed output, this will be bubbled up through
///       [`std::io::Error`]. For more details, see the source code for this function.
pub async fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    /// Read a line out of `reader` or break with [`CloseReason`].
    ///
    /// Implicitly calls `.await`.
    ///
    /// # Breaks
    ///
    /// If `read_line` reads zero bytes, `break` with [`CloseReason::ClosedByClient`].
    /// If `read_line` takes more than [`timeouts::SERVER_TIMEOUT`], break with
    /// [`CloseReason::TimedOut`].
    ///
    /// # Errors
    ///
    /// - Any errors that could come out of the supplied reader's `read_line` function.
    macro_rules! read_line_or_break {
        ($reader:expr) => {
            match ::tokio::time::timeout(
                $crate::timeouts::SERVER_TIMEOUT,
                $crate::read_line!($reader),
            )
            .await
            {
                Ok(result) => match result {
                    Ok(line) => Ok(line),
                    Err(err) => match err.kind() {
                        ::std::io::ErrorKind::ConnectionAborted => {
                            break CloseReason::ClosedByClient
                        }
                        err => Err(err),
                    },
                },
                Err(elapsed) => break CloseReason::TimedOut(elapsed),
            }
        };
    }

    // The errors involved here are not documented. After an extraordinary romp through `tokio`,
    // `mio`, `std`, `core`, and `libc`, I have identified two sources of errors.
    //
    // On Unix, this all wraps `getsockname` and `getpeername` from the C standard library.
    // Other platforms may vary; pull requests to update documentation are welcome.
    //
    // Errors come from two places:
    //
    // - Errors from `get*name` themselves. If `get*name` returns a status code of `-1`, the
    //   will retrieved by [`std::io::Error::last_os_error`].
    // - Errors from malformed output by `get*name`. If `get*name` receives something other than an
    //   IPv4 or IPv6 address, it will return a [`std::io::Error`] with
    //   [`std::io::ErrorKind::InvalidInput`] and `"invalid argument"`.
    //
    // POSIX.1-2008:
    //
    // - <https://pubs.opengroup.org/onlinepubs/9799919799.2024edition/functions/getsockname.html>
    // - <https://pubs.opengroup.org/onlinepubs/9799919799.2024edition/functions/getpeername.html>
    let local_socket = stream.local_addr()?;
    let client_socket = stream.peer_addr()?;
    println!("Connection opened on {local_socket} by {client_socket}");

    let (read_stream, mut write_stream) = stream.split();
    let mut reader = BufReader::new(read_stream);

    write_fmt_line!(write_stream, "220 {DOMAIN} SMTP testing service ready")?;

    let close_reason = loop {
        let line = read_line_or_break!(reader)?;

        match command::handle(&mut write_stream, line).await? {
            ShouldClose::Close(reason) => break reason,
            ShouldClose::Keep => (),
        }
    };

    println!("Connection on {local_socket} with {client_socket} closed ({close_reason:?})");
    Ok(())
}

/// Indicates if and why a TCP connection should be closed.
#[derive(PartialEq, Eq, Debug)]
enum ShouldClose {
    /// The TCP connection should be kept open.
    Keep,
    /// The TCP connection should be closed because [`CloseReason`].
    Close(CloseReason),
}

/// Indicates why a TCP connection should be closed.
#[derive(PartialEq, Eq, Debug)]
#[expect(dead_code)]
enum CloseReason {
    /// The SMTP client requested to quit the session.
    Quit,
    /// An error occurred in the implementation.
    Error,
    /// More time [`Elapsed`] than [`crate::timeouts::SERVER_TIMEOUT`] specifies.
    TimedOut(Elapsed),
    /// The TCP connection was forcefully ended by the client.
    ClosedByClient,
}
