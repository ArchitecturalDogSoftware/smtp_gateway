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

use futures_util::{pin_mut, StreamExt};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::{read_line, write_line};

mod is_valid_response;

type Result = std::result::Result<(), Box<dyn Error>>;

// 4.5.1 Minimum Implementation:
//
// - [ ] `EHLO`
// - [x] `HELO`
// - [ ] `MAIL`
// - [ ] `RCPT`
// - [ ] `DATA`
// - [ ] `RSET`
// - [ ] `NOOP`
// - [ ] `VRFY`
// - [ ] `QUIT`
//
// <https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.1>
#[tokio::test]
async fn test_listen() -> Result {
    const ADDR: &str = "127.0.0.1:8080";

    let stream = crate::listen(TcpListener::bind(ADDR).await?);

    // Can be bound to a variable which exposes `.abort()`
    tokio::spawn(async move {
        pin_mut!(stream);

        loop {
            // Get the `Next` and unwrap it
            let session = stream
                .next()
                .await
                .unwrap()
                // Unwrap the [`TcpListener::accept`]
                .unwrap()
                // Await and unwrap the [`JoinHandle`]
                .await
                .unwrap();

            // Unwrap the [`Session`] itself
            session.unwrap();
        }
    });

    let mut stream = TcpStream::connect(ADDR).await?;
    let (read_stream, mut write_stream) = stream.split();

    let mut reader = BufReader::new(read_stream);

    assert!(is_valid_response::server_greeting(
        &read_line!(reader).await?
    ));

    write_line!(write_stream, "HELO")?;
    assert!(is_valid_response::helo(&read_line!(reader).await?));

    write_line!(write_stream, "QUIT")?;
    assert!(is_valid_response::quit(&read_line!(reader).await?));

    Ok(())
}
