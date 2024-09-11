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

#[tokio::test]
async fn test_listen() -> Result {
    const ADDR: &str = "127.0.0.1:8080";

    // TODO: this should be CRLF, not LF
    const MSG: &str = "Hello TCP!
s
  d
az
";

    // Can be bound to a variable which exposes `.abort()`
    tokio::spawn(crate::listen(TcpListener::bind(ADDR).await?));

    let mut stream = TcpStream::connect(ADDR).await?;
    let (read_stream, mut write_stream) = stream.split();

    let mut reader = BufReader::new(read_stream);
    let mut response = String::new();

    write_stream.write_all(MSG.as_bytes()).await?;

    // Never-ending
    // Use with `cargo test -- --nocapture` for an accessible server to test with utilities
    // like `nc` or `telnet`
    loop {
        let read_bytes = reader.read_line(&mut response).await?;

        if read_bytes == 0 {
            break;
        }
    }

    assert_eq!(MSG, response);

    Ok(())
}
