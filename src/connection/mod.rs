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

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::error::Elapsed,
};

use crate::{str::CRLF, write_fmt_line, write_line};

const DOMAIN: &str = "example.com";

/// Handle a TCP connection as an SMTP session.
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

    println!(
        "Connection opened on {} by {}",
        stream.local_addr()?,
        stream.peer_addr()?
    );

    let (read_stream, mut write_stream) = stream.split();
    let mut reader = BufReader::new(read_stream);

    write_fmt_line!(write_stream, "220 {DOMAIN} SMTP Testing Service Read")?;

    let close_reason = loop {
        let line = read_line_or_break!(reader)?;

        match handle_smtp_command(&mut write_stream, line).await? {
            ShouldClose::Close(reason) => break reason,
            ShouldClose::Keep => (),
        }
    };

    println!(
        "Connection on {} with {} closed ({close_reason:?})",
        stream.local_addr()?,
        stream.peer_addr()?
    );

    Ok(())
}

/// Reply to a line from the client in an SMTP session.
///
/// # Errors
///
/// Whatever errors [`write_line`] may return.
async fn handle_smtp_command(
    write_stream: &mut tokio::net::tcp::WriteHalf<'_>,
    line: String,
) -> Result<ShouldClose, std::io::Error> {
    // RFC 5321 section 2.3.8 specifies that lines ending with anything other than `CRLF` must not
    // be recognized.
    //
    // https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3.8
    if !line.ends_with(CRLF) {
        write_line!(write_stream, "500 Syntax error - no trailing CRLF")?;
        return Ok(ShouldClose::Keep); // Should this close the session?
    }

    // Trim whitespace from line.
    //
    // RFC 5321 section 4.1.1 recommends to allow for trailing whitespace.
    // This trims leading whitespace as well, for the sake of Postel's Law.
    //
    // https://www.rfc-editor.org/rfc/rfc5321.html#section-4.1.1
    let trimmed = line.trim();
    // Extract the command per RFC 5321 section 2.4.
    //
    // Note that the mailbox-local part of an email address (ex. `smith` in `smith@example.com`) is
    // the only case-sensitive part of an SMTP command, so `text` is not set to uppercase.
    //
    // https://www.rfc-editor.org/rfc/rfc5321.html#section-2.4
    let (command, text) = match trimmed.split_once(' ') {
        Some((c, t)) => (c.to_uppercase(), Some(t)),
        None => (trimmed.to_uppercase(), None),
    };

    if command == "QUIT" {
        write_line!(write_stream, "221 Bye")?;
        return Ok(ShouldClose::Close(CloseReason::Quit));
    }

    Ok(ShouldClose::Keep)
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
