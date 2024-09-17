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

//! Handles responding to a particular commands from SMTP clients.

use std::io::Result;

use tokio::{io::AsyncWriteExt, net::tcp::WriteHalf};

use super::{
    super::{CloseReason, ShouldClose},
    Command,
};
use crate::{write_fmt_line, write_line};

/// Send a `"500 Syntax error - {}"` reply into `write_stream` and return with
/// [`ShouldClose::Keep`].
///
/// # Errors
///
/// - Any errors that could come out of the supplied reader's `write_all` function.
macro_rules! syntax_err_and_return {
    ( $write_stream:expr, $error:expr ) => {{
        $crate::write_fmt_line!($write_stream, "500 Syntax error - {}", $error)?;
        return Ok(ShouldClose::Keep); // Should this close the connection?
    }};
}

/// Reply to an unrecognized command from a client.
///
/// See [`not_implemented`] for commands that are recognized, but not implemented. See [RFC 5321
/// section 4.2.4](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.2.4) for more details.
///
/// # Errors
///
/// [`std::io::Error`] from [`AsyncWriteExt::write_all`] on [`tokio::net::TcpStream`].
pub async fn unrecognized(write_stream: &mut WriteHalf<'_>, _: Command) -> Result<ShouldClose> {
    write_fmt_line!(write_stream, "500 Command not recognized")?;

    Ok(ShouldClose::Keep)
}

/// Reply to a command from the client that is recognized but not implemented.
///
/// [RFC 5321 section 4.2.4](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.2.4).
///
/// See [`unrecognized`] for cases of truly unrecognized commands.
///
/// # Errors
///
/// [`std::io::Error`] from [`AsyncWriteExt::write_all`] on [`tokio::net::TcpStream`].
pub async fn not_implemented(write_stream: &mut WriteHalf<'_>, _: Command) -> Result<ShouldClose> {
    write_fmt_line!(write_stream, "502 Command not implemented")?;

    Ok(ShouldClose::Keep)
}

/// Reply to the hello (`HELO`) command from a client.
///
/// [RFC 5321 section 4.1.1.1](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.1.1.1).
///
/// # Errors
///
/// [`std::io::Error`] from [`AsyncWriteExt::write_all`] on [`tokio::net::TcpStream`].
pub async fn hello(write_stream: &mut WriteHalf<'_>, command: Command) -> Result<ShouldClose> {
    todo!()
}

/// Reply to the quit (`QUIT`) command from a client.
///
/// [RFC 5321 section 4.1.1.10](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.1.1.1).
///
/// # Errors
///
/// [`std::io::Error`] from [`AsyncWriteExt::write_all`] on [`tokio::net::TcpStream`].
pub async fn quit(write_stream: &mut WriteHalf<'_>, _: Command) -> Result<ShouldClose> {
    write_line!(write_stream, "221 Bye")?;
    Ok(ShouldClose::Close(CloseReason::Quit))
}
