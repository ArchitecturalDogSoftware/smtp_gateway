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

use std::fmt::Display;

use ascii::{AsAsciiStrError, AsciiString, IntoAsciiString};

pub const CR: char = '\r';
pub const LF: char = '\n';
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
    /// - Any [`CR`] not followed by [`LF`] with [`CRLF`].
    /// - Any [`LF`] not preceded by [`CR`] with [`CRLF`].
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
        let str = replace_all_endings_with_crlf(str);

        Ok(Self {
            str: str.into_ascii_string().map_err(|e| e.ascii_error())?,
        })
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

/// Replaces:
/// - Any [`CR`] not followed by [`LF`] with [`CRLF`].
/// - Any [`LF`] not preceded by [`CR`] with [`CRLF`].
///
/// This means that any `LFCR` (`"\n\r"`) (not including the `"\n\r"` in the middle of
/// `"\r\n\r\n"`) is replaced with `"\r\n\r\n"`.
fn replace_all_endings_with_crlf(str: &str) -> String {
    let mut iter = str.chars();

    let mut previous = ' '; // Dummy value
    let Some(mut current) = iter.next() else {
        return String::new();
    };
    let mut next = iter.next();

    let mut output = String::new();

    loop {
        match current {
            // Push `CR` onto the output and push `LF` if not already up next
            CR => {
                output.push(CR);
                match next {
                    Some(LF) => (),       // Do nothing, `CRLF` is correct
                    _ => output.push(LF), // `CR` -> `CRLF`
                }
            }
            // Push `LF` onto the output, pushing `CR` first if not already present
            LF => {
                match previous {
                    CR => (),             // Do nothing, `CRLF` is correct
                    _ => output.push(CR), // `LF` -> `CRLF`
                }
                output.push(LF);
            }
            // Push onto the output
            c => output.push(c),
        }

        previous = current;
        current = match next {
            Some(c) => c,
            None => break,
        };
        next = iter.next();
    }

    output
}
