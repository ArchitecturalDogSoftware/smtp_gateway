// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2024 RemasteredArch
// Copyright © 2024 Jaxydog
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

use std::{borrow::Cow, fmt::Display};

use ascii::{AsAsciiStr, AsAsciiStrError, AsciiChar, AsciiStr, AsciiString};

pub const CRLF: &str = "\r\n";

/// A string guaranteed for usage with SMTP.
///
/// [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html) requires that only US-ASCII character
/// encoding (sections 2.3.1 and 2.4) and `CRLF` line endings (section 2.3.8) are used.
#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone, Default)]
pub struct SmtpString {
    str: AsciiString,
}

impl SmtpString {
    /// Creates a new [`Self`] from a string containing ASCII characters and fixes non-[`CRLF`]
    /// line endings.
    ///
    /// Replaces:
    /// - Any [`AsciiChar::CarriageReturn`] not followed by [`AsciiChar::LineFeed`] with [`CRLF`].
    /// - Any [`AsciiChar::LineFeed`] not preceded by [`AsciiChar::CarriageReturn`] with [`CRLF`].
    ///
    /// # Errors
    ///
    /// Returns an error if the input string contains invalid ASCII.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use smtp_gateway::str::SmtpString;
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let smtp = SmtpString::new("LF\nCR\rLFCR\n\rCRLF\r\nCRLFCRLF\r\n\r\n")?;
    ///
    /// assert_eq!(smtp.to_string(), "LF\r\nCR\r\nLFCR\r\n\r\nCRLF\r\nCRLFCRLF\r\n\r\n");
    /// #     Ok(())
    /// # }
    /// ```
    pub fn new(str: &str) -> Result<Self, AsAsciiStrError> {
        let str = str.as_ascii_str()?;
        let str = self::replace_endings_with_crlf(str).into_owned();

        Ok(Self { str })
    }

    /// Create a [`Self`] from an [`AsciiString`].
    ///
    /// # Safety
    ///
    /// The [`AsciiString`] is not checked for proper usage of `CRLF` (`"\r\n"`) line endings. It is
    /// up to the consumer to ensure that it does not violate the rules of [RFC 5321 section
    /// 2.3.8](https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3.8).
    #[must_use]
    pub const unsafe fn from_ascii_str_unchecked(str: AsciiString) -> Self {
        Self { str }
    }

    /// Return a reference to the inner [`AsciiString`].
    #[must_use]
    pub const fn as_inner(&self) -> &AsciiString {
        &self.str
    }

    /// Return a reference to the contents as their raw byte representations.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.str.as_bytes()
    }
}

impl Display for SmtpString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.str.fmt(f)
    }
}

/// Replaces all line endings in the given string with `CRLF`-style endings (`"\r\n"`).
///
/// This will preserve pre-existing `"\r\n"` characters while replacing the following cases:
/// - `'\r'` -> `"\r\n"`
/// - `'\n'` -> `"\r\n"`
/// - `"\n\r"` -> `"\r\n\r\n"`
///
/// If the original string does not need to be modified, this function will not allocate.
fn replace_endings_with_crlf(string: &AsciiStr) -> Cow<AsciiStr> {
    let mut output = Cow::Borrowed(string);
    let mut previous = None;

    #[expect(clippy::iter_skip_zero, reason = "Needed to preserve type integrity")]
    let mut iterator = output.chars().enumerate().skip(0).peekable();

    while let Some((index, character)) = iterator.next() {
        match character {
            // If the previous character is not a carriage return.
            AsciiChar::LineFeed if !matches!(previous, Some(AsciiChar::CarriageReturn)) => {
                // Insert one before this.
                output.to_mut().insert(index, AsciiChar::CarriageReturn);
            }
            // If the next character is not a line feed.
            AsciiChar::CarriageReturn
                if !matches!(iterator.peek(), Some((_, AsciiChar::LineFeed))) =>
            {
                // Insert one after this.
                output.to_mut().insert(index + 1, AsciiChar::LineFeed);
            }
            // Ignore any other characters.
            _ => {
                previous = Some(character);

                continue;
            }
        }

        // Skip over all previous characters *and* the added one.
        // This is needed to update the iterator after changing the string.
        iterator = output.chars().enumerate().skip(index + 2).peekable();
        // The previous character after modifications should always be a line feed.
        previous = Some(AsciiChar::LineFeed);
    }

    output
}
