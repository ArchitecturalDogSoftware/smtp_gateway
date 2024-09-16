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

//! Handles responding to a command from an SMTP client.
//!
//! See [`handle`].

use tokio::io::AsyncWriteExt;

use super::{CloseReason, ShouldClose};
use crate::{str::CRLF, write_line};

/// Reply to a line from the client in an SMTP session.
///
/// # Errors
///
/// Whatever errors [`write_line`] may return.
pub async fn handle(
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

    // RFC 5321 uses US-ASCII, specifically ANSI X3.4-1968 (reference 6).
    // As far as I can tell, [`std::ascii:Char`] upholds a standard that is functionally equivalent
    // for the purposes of this library.
    //
    // https://www.rfc-editor.org/rfc/rfc5321.html#ref-6
    if !line.is_ascii() {
        write_line!(write_stream, "500 Syntax error - invalid character")?;
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
    let (command, text) = match trimmed.split_once([' ', '-']) {
        Some((c, t)) => (c.to_ascii_uppercase(), Some(t)),
        None => (trimmed.to_ascii_uppercase(), None),
    };

    let is_multiline = match trimmed
        .chars()
        .nth(command.chars().count())
        .expect("a string must contain a substring of itself")
    {
        '-' => MultiLine::HasNext,
        ' ' => MultiLine::LastLine,
        _ => unreachable!("`command` will only split on `' '` or `'-'`"),
    };

    if command == "QUIT" {
        write_line!(write_stream, "221 Bye")?;
        return Ok(ShouldClose::Close(CloseReason::Quit));
    }

    Ok(ShouldClose::Keep)
}

/// Indicates if the parsed command is the last line to be parsed before replying.
#[derive(PartialEq, Eq, Debug)]
enum MultiLine {
    /// This is the last line to be parsed before replying.
    LastLine,
    /// This is not the last line to be parsed before replying, there will be more incoming.
    HasNext,
}

impl MultiLine {
    /// Get the character used to split the verb and text of an SMTP command.
    #[must_use]
    pub const fn split(&self) -> char {
        match self {
            Self::LastLine => ' ',
            Self::HasNext => '-',
        }
    }
}
