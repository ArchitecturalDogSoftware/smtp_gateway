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

use std::{
    fmt::{Debug, Display},
    ops::Range,
};

use ascii::{AsciiStr, AsciiString, IntoAsciiString};
use tokio::io::AsyncWriteExt;

use super::{CloseReason, ShouldClose};
use crate::{str::CRLF, write_line};

/// Reply to a line from the client in an SMTP session.
///
/// # Errors
///
/// [`std::io::Error`] from [`AsyncWriteExt::write_all`] on [`tokio::net::TcpStream`].
pub async fn handle(
    write_stream: &mut tokio::net::tcp::WriteHalf<'_>,
    line: String,
) -> std::io::Result<ShouldClose> {
    /// Send a `"500 Syntax error - {}"` reply into `write_stream` and return with
    /// [`ShouldClose::Keep`].
    ///
    /// # Errors
    ///
    /// - Any errors that could come out of the supplied reader's `read_line` function.
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

        // If `end < start` or `start == end`.
        if range.is_empty() {
            None
        } else {
            Some(range)
        }
    }

    /// Extract the command per RFC 5321 section 2.4.
    ///
    /// <https://www.rfc-editor.org/rfc/rfc5321.html#section-2.4>
    fn split_command(command: &AsciiStr) -> (Range<usize>, Option<Range<usize>>, MultiLine) {
        let (verb, text) = match command.as_str().split_once([' ', '-']) {
            Some((verb, _text)) => (
                // From the start until the last byte of verb.
                0..verb.len(),
                // `verb.len()` would point towards the character that was split on, so start at
                // the byte *after* that and end at the last byte.
                Some(verb.len() + 1..command.len()),
            ),
            None => (0..command.len(), None),
        };

        let multiline_type = match command.chars().nth(verb.len()) {
            Some(ascii::AsciiChar::Minus) => MultiLine::HasNext,
            Some(ascii::AsciiChar::Space) | None => MultiLine::LastLine,
            _ => unreachable!("`command` will only split on `' '` or `'-'`"),
        };

        (verb, text, multiline_type)
    }

    if line.is_empty() {
        return Err(CommandError::Empty);
    }

    // Will not error because of emptiness, as this was already checked above.
    let trimmed = trim(&line).ok_or(CommandError::OnlyWhitespace)?;
    let trimmed_str = &line[trimmed.clone()];

    let (verb, text, multiline) = split_command(trimmed_str);

    // These ranges were obtained using the trimmed string instead of the actual line. This
    // recalibrates the ranges to point to their locations on the actual line instead of on the
    // trimmed string.
    let adjust_for_trim = |mut range: Range<usize>| {
        range.start += trimmed.start;
        range.end += trimmed.start;

        range
    };
    let verb = adjust_for_trim(verb);
    let text = text.map(adjust_for_trim);

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

/// One line of an SMTP command.
#[derive(PartialEq, Eq, Clone)]
struct Command {
    /// The entire line, unmodified except for the [`Self::verb`] range being set to uppercase.
    line: AsciiString,
    /// The range over [`Self::line`] without leading and trailing whitespace.
    trimmed: Range<usize>,
    /// The range over [`Self::line`] containing the verb of the command.
    verb: Range<usize>,
    /// The range over [`Self::line`] containing the text of the command.
    text: Option<Range<usize>>,
    /// The [`MultiLine`] type of the command.
    ///
    /// Derived from the character that [`Self::verb`] and [`Self::text`] were split by.
    multiline: MultiLine,
}

// Consuming implementation is not complete
impl Command {
    /// Get the entire line as a string slice, unmodified unmodified except for the [`Self::verb`]
    /// range being set to uppercase.
    pub fn line(&self) -> &AsciiStr {
        self.line.as_ref()
    }

    /// Get the line with leading and trailing whitespace stripped as a string slice.
    pub fn trimmed(&self) -> &AsciiStr {
        self.get(&self.trimmed)
    }

    /// Get the verb of the command as an uppercase string slice.
    pub fn verb(&self) -> &AsciiStr {
        self.get(&self.verb)
    }

    /// Get the text of the command as a string slice.
    pub fn text(&self) -> Option<&AsciiStr> {
        let range = self.text.as_ref()?;

        Some(self.get(range))
    }

    /// Get the [`MultiLine`] type of the command.
    ///
    /// Derived from the character that [`Self::verb`] and [`Self::text`] were split by.
    pub const fn multiline(&self) -> MultiLine {
        self.multiline
    }

    /// Get a range of the internal [`AsciiString`] as a string slice.
    fn get(&self, range: &Range<usize>) -> &AsciiStr {
        &self.line[range.clone()]
    }
}

impl Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("line", &self.line)
            .field("line()", &self.line())
            .field("trimmed", &self.trimmed)
            .field("trimmed()", &self.trimmed())
            .field("verb", &self.verb)
            .field("verb()", &self.verb())
            .field("text", &self.text)
            .field("text()", &self.text())
            .field("multiline", &self.multiline)
            .field("multiline()", &self.multiline())
            .finish()
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
    #[expect(dead_code)]
    #[must_use]
    pub const fn split(self) -> char {
        match self {
            Self::LastLine => ' ',
            Self::HasNext => '-',
        }
    }
}

/// Possible error states encountered when trying to convert a line into a [`Command`].
#[derive(PartialEq, Eq, Copy, Clone)]
enum CommandError {
    /// Function was passed a line that is empty.
    Empty,
    /// Function was passed a line that consists of only whitespace.
    OnlyWhitespace,
}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Empty => "empty command",
            Self::OnlyWhitespace => "command consists only of whitespace",
        })
    }
}

impl Debug for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self} at {} {}", file!(), line!())
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

#[cfg(test)]
mod test {
    use ascii::AsAsciiStr;

    use super::*;

    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_command_parsing() -> Result {
        let command = parse("  foo bar baz bim  \r\n".into_ascii_string()?)?;

        // Tests that it constructs the right object.
        assert_eq!(
            command,
            Command {
                line: "  FOO bar baz bim  \r\n".into_ascii_string()?,
                trimmed: 2..17,    // `"FOO bar baz bim"`.
                verb: 2..5,        // "`FOO`".
                text: Some(6..17), // "`bar baz bim`".
                multiline: MultiLine::LastLine,
            }
        );

        // Tests that it produces the right strings.
        assert_eq!(command.line(), "  FOO bar baz bim  \r\n".as_ascii_str()?);
        assert_eq!(command.trimmed(), "FOO bar baz bim".as_ascii_str()?);
        assert_eq!(command.verb(), "FOO".as_ascii_str()?);
        assert_eq!(command.text(), Some("bar baz bim".as_ascii_str()?));

        // Tests that it does not perform any `CRLF` checks.
        assert_eq!(
            parse("foo bar\n".into_ascii_string()?)?.line(),
            "FOO bar\n".as_ascii_str()?
        );

        // Test for handling of no text.
        assert_eq!(
            parse("foo\r\n".into_ascii_string()?)?,
            Command {
                line: "FOO\r\n".into_ascii_string()?,
                trimmed: 0..3,
                verb: 0..3,
                text: None,
                multiline: MultiLine::LastLine,
            }
        );

        // Test that having a space but no text after the verb still counts as no text.
        assert_eq!(
            parse("foo \r\n".into_ascii_string()?)?,
            Command {
                line: "FOO \r\n".into_ascii_string()?,
                trimmed: 0..3,
                verb: 0..3,
                text: None,
                multiline: MultiLine::LastLine,
            }
        );

        Ok(())
    }
}
