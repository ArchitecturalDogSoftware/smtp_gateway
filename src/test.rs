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

use std::error::Error;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

type Result = std::result::Result<(), Box<dyn Error>>;

/// Read a line out of `reader`.
///
/// # Errors
///
/// - Any errors that would come out of the supplied reader's `read_line` function.
/// - If `read_line` reads zero bytes, [`std::io::ErrorKind::ConnectionAborted`] is returned.
macro_rules! read_line {
    ($reader:expr) => {{
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
    }};
}

/// Write a string literal into `writer` as an [`crate::SmtpStr`]. Appends a line ending.
///
/// # Errors
///
/// - Any errors that would come out of the supplied writer's `write_all` function.
/// - Forcefully returns with `?` if passed invalid ASCII.
///     - This is undesirable, but a limitation of the current implementation.
macro_rules! write_line {
    ($writer:expr, $str:expr) => {{
        $crate::smtp_str!(write_line_macro_str = $str);
        $writer.write_all(write_line_macro_str.as_bytes()).await
    }};
}

#[tokio::test]
async fn test_listen() -> Result {
    const ADDR: &str = "127.0.0.1:8080";

    // Can be bound to a variable which exposes `.abort()`
    tokio::spawn(crate::listen(TcpListener::bind(ADDR).await?));

    let mut stream = TcpStream::connect(ADDR).await?;
    let (read_stream, mut write_stream) = stream.split();

    let mut reader = BufReader::new(read_stream);

    assert!(is_valid_server_greeting(&read_line!(reader)?));

    write_line!(write_stream, "HELO")?;
    assert!(is_valid_helo_response(&read_line!(reader)?));

    Ok(())
}

/// Checks if the server's opening message roughly matches [RFC 5321,
/// section 4.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.2).
///
/// Considers a 554 response to be an error.
fn is_valid_server_greeting(str: &str) -> bool {
    str.starts_with("220") && is_valid_smtp_line(str)
}

/// Checks if the server's opening message roughly matches [RFC 5321,
/// section 4.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.2).
///
/// Considers a 554 response to be an error.
fn is_valid_helo_response(str: &str) -> bool {
    todo!() && is_valid_smtp_line(str)
}

/// Checks whether a string is ASCII and ends with `CRLF`.
///
/// [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html) requires that only US-ASCII character
/// encoding (sections 2.3.1 and 2.4) and `CRLF` line endings (section 2.3.8) are used.
#[inline]
fn is_valid_smtp_line(str: &str) -> bool {
    str.ends_with("\r\n") && str.is_ascii()
}
