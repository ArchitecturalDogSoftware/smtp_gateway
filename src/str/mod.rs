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

pub(crate) mod max_lengths;
#[cfg(test)]
mod test;

pub const CRLF: &str = "\r\n";
pub const MAX_LEN: usize = 150;

/// A string guaranteed for usage with SMTP.
///
/// [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html) requires that only US-ASCII character
/// encoding (sections 2.3.1 and 2.4) and `CRLF` line endings (section 2.3.8) are used.
///
/// Methods do not append a trailing line ending sequence. This creates strings, not necessarily
/// full lines.
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
    /// Does not append a trailing line ending sequence.
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

/// A fixed-length, stack-allocated string that is expected to be used like [`SmtpString`].
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone)]
pub(crate) struct RawSmtpStr {
    pub buffer: [AsciiChar; MAX_LEN],
    pub len: usize,
}

#[expect(dead_code, reason = "not finished yet")]
impl RawSmtpStr {
    /// Constructs a new [`Self`] with the buffer filled with [`AsciiChar::_0`] and len
    /// `0`.
    pub const fn new_zeroed() -> Self {
        Self {
            buffer: [AsciiChar::_0; MAX_LEN],
            len: 0,
        }
    }

    /// Replaces all line endings in the given string with `CRLF`-style endings (`"\r\n"`) without
    /// allocating to the heap.
    ///
    /// Intended to be used alongside a function or macro to convert [`Self`] into a more
    /// appropriate type.
    ///
    /// This will preserve pre-existing `"\r\n"` characters while replacing the following cases:
    /// - `'\r'` -> `"\r\n"`
    /// - `'\n'` -> `"\r\n"`
    /// - `"\n\r"` -> `"\r\n\r\n"`
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Provided invalid ASCII.
    /// - The input or output strings are longer than [`MAX_LEN`] bytes.
    pub const fn new(str: &str) -> Self {
        if str.is_ascii() {
            let str = {
                let bytes = std::ptr::from_ref::<[u8]>(str.as_bytes());

                // Safety: we just verified that `str.is_ascii`.
                unsafe { &*(bytes as *const AsciiStr) }
            };
            Self::new_from_ascii(str)
        } else {
            panic!("provided invalid ASCII")
        }
    }

    /// Replaces all line endings in the given string with `CRLF`-style endings (`"\r\n"`) without
    /// allocating to the heap.
    ///
    /// Intended to be used alongside a function or macro to convert [`Self`] into a more
    /// appropriate type.
    ///
    /// This will preserve pre-existing `"\r\n"` characters while replacing the following cases:
    /// - `'\r'` -> `"\r\n"`
    /// - `'\n'` -> `"\r\n"`
    /// - `"\n\r"` -> `"\r\n\r\n"`
    ///
    /// # Panics
    ///
    /// Panics if the input or output strings are longer than [`MAX_LEN`] bytes.
    pub const fn new_from_ascii(string: &AsciiStr) -> Self {
        assert!(string.len() <= MAX_LEN);

        let slice = string.as_slice();
        let mut output = Self::new_zeroed();

        let mut previous: Option<AsciiChar> = None;
        // Tracks the position in [`string`].
        let mut index: usize = 0;
        // Tracks the corresponding position in [`output`].
        let mut output_index: usize = 0;

        while index < string.len() {
            let char = slice[index];

            match char {
                // If the previous character is not a carriage return.
                AsciiChar::LineFeed if !matches!(previous, Some(AsciiChar::CarriageReturn)) => {
                    // Insert one before this.
                    output.buffer[output_index] = AsciiChar::CarriageReturn;
                    output_index += 1;
                    output.len += 1;
                    output.buffer[output_index] = char;
                }
                // If the next character is not a line feed.
                AsciiChar::CarriageReturn
                    if !(
                        // Out of bounds check to avoid panicking on strings that are of valid length,
                        // but just end with carriage return.
                        index + 1 < slice.len() &&
                        // If next character is a line feed.
                        matches!(slice[index + 1], AsciiChar::LineFeed)
                    ) =>
                {
                    // Insert one after this.
                    output.buffer[output_index] = char;
                    output_index += 1;
                    output.len += 1;
                    output.buffer[output_index] = AsciiChar::LineFeed;
                }
                _ => {
                    output.buffer[output_index] = char;
                }
            }

            output.len += 1;

            previous = Some(char);
            index += 1;
            output_index += 1;
        }

        output
    }

    /// Gets the stored string ([`Self::buffer`] from 0..[`Self::len`]) as a string slice.
    pub fn as_str(&self) -> &str {
        self.as_ascii_str().as_str()
    }

    /// Gets the stored string ([`Self::buffer`] from 0..[`Self::len`]) as an [`AsciiStr`].
    pub fn as_ascii_str(&self) -> &AsciiStr {
        // Safety: a slice of [`AsciiChar`] is exactly how [`AsciiStr`] is represented.
        unsafe { self.as_slice().as_ascii_str_unchecked() }
    }

    /// Gets the stored string ([`Self::buffer`] from 0..[`Self::len`]) as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.as_ascii_str().as_bytes()
    }

    /// Get a slice of [`Self::buffer`] from 0..[`Self::len`] (the stored string).
    pub fn as_slice(&self) -> &[AsciiChar] {
        &self.buffer[..self.len]
    }

    /// Get the length of the stored string.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Get the length of the internal buffer.
    pub const fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Unwrap [`Self`] into a tuple holding the inner buffer and the length of the stored string.
    pub(crate) const fn into_inner(self) -> ([AsciiChar; MAX_LEN], usize) {
        (self.buffer, self.len)
    }

    /// Consume [`Self`] to create an [`SmtpString`].
    pub fn into_smtp_string(self) -> SmtpString {
        // Safety: [`Self::new_from_ascii`] already ensures CRLF.
        unsafe { SmtpString::from_ascii_str_unchecked(self.as_ascii_str().to_ascii_string()) }
    }
}
