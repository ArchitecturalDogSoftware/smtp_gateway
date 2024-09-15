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

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

const DOMAIN: &str = "example.com";

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

    write_stream
        .write_all(format!("220 {DOMAIN} SMTP Testing Service Ready\r\n").as_bytes())
        .await?;

    let close_reason = loop {
        let line = read_line_or_break!(reader)?;
        let (response, should_close) = handle_smtp_command(&line);

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

fn handle_smtp_command(line: &str) -> (&str, ShouldClose) {
    // Trim whitespace from line
    let trimmed = line.trim();

    if trimmed == "QUIT" {
        return ("221 Bye\r\n", ShouldClose::Close(CloseReason::Quit));
    }

    (line, ShouldClose::Keep)
}

#[derive(PartialEq, Eq, Debug)]
enum ShouldClose {
    Keep,
    Close(CloseReason),
}

#[derive(PartialEq, Eq, Debug)]
#[expect(dead_code)]
enum CloseReason {
    Quit,
    Error,
    TimedOut(tokio::time::error::Elapsed),
    ClosedByClient,
}
