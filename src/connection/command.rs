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

use std::{fmt::Display, ops::Range};

use ascii::{AsciiStr, AsciiString, IntoAsciiString};
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
    /// Send an 500 syntax error reply into `write_stream` and return with [`ShouldClose::Keep`].
    macro_rules! err_and_return {
        ( $write_stream:expr, $error:expr ) => {{
            $crate::write_fmt_line!($write_stream, "500 Syntax error - {}", $error)?;
            return Ok(ShouldClose::Keep); // Should this close the connection?
        }};
    }

    // RFC 5321 section 2.3.8 specifies that lines ending with anything other than `CRLF` must not
    // be recognized.
    //
    // https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3.8
    if !line.ends_with(CRLF) {
        err_and_return!(write_stream, "no trailing CRLF");
    }

    // RFC 5321 uses US-ASCII, specifically ANSI X3.4-1968 (reference 6).
    // As far as I can tell, [`std::ascii:Char`] upholds a standard that is functionally equivalent
    // for the purposes of this library.
    //
    // https://www.rfc-editor.org/rfc/rfc5321.html#ref-6
    let Ok(line) = line.into_ascii_string() else {
        err_and_return!(write_stream, "invalid character");
    };

    let command = match parse(line) {
        Ok(c) => c,
        Err(e) => err_and_return!(write_stream, e),
    };

    let verb = command.verb();

    if verb == "QUIT" {
        write_line!(write_stream, "221 Bye")?;
        return Ok(ShouldClose::Close(CloseReason::Quit));
    }

    Ok(ShouldClose::Keep)
}

/// Parse a line as a command.
fn parse(mut line: AsciiString) -> Result<Command, CommandError> {
    /// Trim the line of leading and trailing whitespace.
    ///
    /// RFC 5321 section 4.1.1 recommends to allow for trailing whitespace.
    /// This trims leading whitespace as well, for the sake of Postel's Law.
    ///
    /// Returns `None` if the string is empty or only whitespace.
    ///
    /// <https://www.rfc-editor.org/rfc/rfc5321.html#section-4.1.1>
    fn trim(str: &AsciiStr) -> Option<Range<usize>> {
        // The index of the first byte that isn't whitespace.
        let leading_whitespace_len = str
            .as_str()
            .find(|c: char| !c.is_ascii_whitespace())
            .unwrap_or(str.len());
        // The index after the last byte that isn't whitespace.
        let trailing_whitespace_len = str.trim_end().len();

        // Convert the indices into a range.
        let range = leading_whitespace_len..trailing_whitespace_len;

        if range.is_empty() {
            None
        } else {
            Some(range)
        }
    }

    /// Extract the command per RFC 5321 section 2.4.
    ///
    /// <https://www.rfc-editor.org/rfc/rfc5321.html#section-2.4>
    fn split_command(command: &AsciiStr) -> (Range<usize>, Option<Range<usize>>) {
        match command.as_str().split_once([' ', '-']) {
            Some((verb, _text)) => (0..verb.len(), Some(verb.len()..command.len())),
            None => (0..command.len(), None),
        }
    }

    if line.is_empty() {
        return Err(CommandError::Empty);
    }

    // Will only ever be because of whitespace because it is checked for emptiness earlier.
    let trimmed = trim(&line).ok_or(CommandError::OnlyWhitespace)?;
    let trimmed_str = &line[trimmed.clone()];

    let (verb, text) = split_command(trimmed_str);
    let multiline = match trimmed_str
        .chars()
        .nth(verb.len())
        .expect("a string must contain a substring of itself")
    {
        ascii::AsciiChar::Minus => MultiLine::HasNext,
        ascii::AsciiChar::Space => MultiLine::LastLine,
        _ => unreachable!("`command` will only split on `' '` or `'-'`"),
    };

    // Make the command verb uppercase for standardized comparison.
    //
    // Note that the mailbox-local part of an email address (ex. `smith` in `smith@example.com`) is
    // the only case-sensitive part of an SMTP command, so `text` is not be set to uppercase.
    let verb_str: &mut AsciiStr = line[verb.clone()].as_mut();
    verb_str.make_ascii_uppercase();

    Ok(Command {
        line,
        trimmed,
        verb,
        text,
        multiline,
    })
}

struct Command {
    line: AsciiString,
    trimmed: Range<usize>,
    verb: Range<usize>,
    text: Option<Range<usize>>,
    multiline: MultiLine,
}

impl Command {
    pub fn line(&self) -> &AsciiStr {
        self.line.as_ref()
    }

    pub fn trimmed(&self) -> &AsciiStr {
        self.get(&self.trimmed)
    }

    pub fn verb(&self) -> &AsciiStr {
        self.get(&self.verb)
    }

    pub fn text(&self) -> Option<&AsciiStr> {
        let range = self.text.as_ref()?;

        Some(self.get(range))
    }

    pub fn multiline(&self) -> MultiLine {
        self.multiline
    }

    fn get(&self, range: &Range<usize>) -> &AsciiStr {
        &self.line[range.clone()]
    }
}

/// Indicates if the parsed command is the last line to be parsed before replying.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
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

/// Possible error states encountered when trying to convert a line into a [`Command`].
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum CommandError {
    /// Function was passed a line that is empty.
    Empty,
    /// Function was passed a line that consists of only whitespace.
    OnlyWhitespace,
}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CommandError::Empty => "empty command",
            CommandError::OnlyWhitespace => "command consists only of whitespace",
        })
    }
}
