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

#![expect(
    clippy::module_name_repetitions,
    reason = "this will be publicly exported outside of this module"
)]

use std::fmt::Display;

use ascii::{AsAsciiStr, AsAsciiStrError, AsciiStr};

const CR: char = '\r';
const LF: char = '\n';

// This is actually despicable. This type should just be a transparent wrapper over an
// [`AsciiString`] or a reference/smart pointer to [`AsciiStr]`.
/// A string guaranteed for usage with SMTP.
///
/// [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html) requires that only US-ASCII character
/// encoding (sections 2.3.1 and 2.4) and `CRLF` line endings (section 2.3.8) are used.
#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct SmtpStr {
    str: AsciiStr,
}

impl SmtpStr {
    /// For a string containing only ASCII characters, replace all instances of `LF` (`'\n'`) with
    /// `CRLF` (`"\r\n"`).
    ///
    /// # Errors
    ///
    /// Returns an error if the input string contains invalid ASCII.
    ///
    /// # Bugs
    ///
    /// While this fixes lone `LF`s, [SMTP](https://www.rfc-editor.org/rfc/rfc5321.html) considers
    /// a lone `CR` to be invalid as well. How should they be handled? Should they be removed or
    /// turned it `CRLF` as well?
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use smtp_gateway::SmtpStr;
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let mut input = "LF\nCRLF\r\n".to_string();
    /// let smtp = SmtpStr::mutate_into(&mut input)?;
    ///
    /// assert_eq!(smtp.as_bytes(), "LF\r\nCRLF\r\n".as_bytes());
    /// #     Ok(())
    /// # }
    /// ```
    pub fn mutate_into(str: &mut String) -> Result<&Self, AsAsciiStrError> {
        // Where `LF` (but not `CRLF`) is encountered in `str`
        let mut lf_indices: Vec<usize> = vec![];
        // The last character, to check that `LF` is not `CRLF`
        let mut previous: char = ' '; // Dummy value

        for (index, char) in str.char_indices() {
            // `LF` but not `CRLF`
            if char == LF && previous != CR {
                lf_indices.push(index);
            }

            previous = char;
        }

        // Insert `CR` before every `LF` to make `CRLF`
        for index in lf_indices {
            str.insert(index, CR);
        }

        Ok(unsafe { Self::from_ascii_str_unchecked(str.as_ascii_str()?) })
    }

    /// Cast an [`AsciiStr`] into a [`Self`].
    ///
    /// # Safety
    ///
    /// The [`AsciiStr`] is not checked for proper usage of `CRLF` (`"\r\n"`) line endings. It is
    /// up to the consumer to ensure that it does not violate the rules of [RFC 5321 section
    /// 2.3.8](https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3.8).
    #[must_use]
    pub const unsafe fn from_ascii_str_unchecked(str: &AsciiStr) -> &Self {
        let ptr = std::ptr::from_ref::<AsciiStr>(str) as *const Self;
        unsafe { &*ptr }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.str.as_bytes()
    }
}

impl Display for SmtpStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.str.fmt(f)
    }
}

pub fn is_smtp_str(str: &str) -> bool {
    fn has_non_crlf(str: &str) -> bool {
        // The last character, to check that `LF` is not `CRLF`
        let mut previous = ' '; // Dummy value

        let mut iter = str.chars();

        while let Some(char) = iter.next() {
            // `LF` but not `CRLF`
            if char == LF && previous != CR {
                return true;
            }

            if char == CR {
                let next_is_lf = iter.next().is_some_and(|c| c == LF);

                if !next_is_lf {
                    return true;
                }
            }

            previous = char;
        }

        false
    }

    str.is_ascii() && !has_non_crlf(str)
}

/// Create an [`SmtpStr`] bound to variable `var` out of string literal `str` with a trailing line
/// ending.
///
/// # Errors
///
/// Calls [`SmtpStr::mutate_into`] with a `?` under the hood. Will return an error if passed an
/// invalid ASCII string.
///
/// # Examples
///
/// ```rust
/// # use smtp_gateway::SmtpStr;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// smtp_str!(smtp = "LF\nCRLF");
///
/// assert_eq!(smtp.as_bytes(), "LF\r\nCRLF\r\n".as_bytes());
/// #     Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! smtp_str {
    ($var:ident = $str:expr) => {
        let $var = &mut concat!($str, "\r\n").to_string();
        let $var = $crate::SmtpStr::mutate_into($var)?;
    };
}
