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

#![warn(clippy::nursery, clippy::pedantic)]
#![cfg_attr(debug_assertions, allow(clippy::missing_errors_doc))]

use std::io;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[macro_use]
mod str;
#[cfg(test)]
mod test;

pub use str::SmtpStr;

const DOMAIN: &str = "example.com";

pub async fn listen(listener: TcpListener) -> io::Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    println!(
        "Connection opened on {} by {}",
        stream.local_addr()?,
        stream.peer_addr()?
    );

    let (read_stream, mut write_stream) = stream.split();
    let mut reader = BufReader::new(read_stream);

    write_stream
        .write_all(format!("220 {DOMAIN} SMTP Testing Service Ready").as_bytes())
        .await?;

    let close_reason = loop {
        // Read a string into a buffer until a newline
        let mut line = String::new();
        let read_bytes = reader.read_line(&mut line).await?;

        // Connection is closed
        if read_bytes == 0 {
            break CloseReason::ClosedByClient;
        }

        let (response, should_close) = handle_smtp_command(&mut line);

        write_stream.write_all(response.as_bytes()).await?;

        match should_close {
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

fn handle_smtp_command(line: &mut String) -> (&str, ShouldClose) {
    // Trim whitespace from line
    let trimmed = line.trim();

    if trimmed == "QUIT" {
        return ("221 Bye\r\n", ShouldClose::Close(CloseReason::Quit));
    }

    (line, ShouldClose::Keep)
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum ShouldClose {
    Keep,
    Close(CloseReason),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
#[expect(dead_code)]
enum CloseReason {
    Quit,
    Error,
    ClosedByClient,
}

/// Tests whether a string is a domain name as considered by SMTP ([RFC 5321, section
/// 2.3.5](https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3.5))
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
