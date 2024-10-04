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

#![expect(dead_code, reason = "kept for thoroughness")]

//! The maximum length, in number of 8-bit bytes, of a variety of items.
//!
//! Note that these are the *minimum* values. SMTP clients and servers must be able to handle at
//! least these limits. They may exceed these limits, but they should be prepared to be rejected by
//! the other party.
//!
//! Per [RFC 5321 section 4.5.3.1](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1).

/// The maximum length of the local-part (such as the username of an email address) in bytes.
///
/// [RFC 5321 § 4.5.3.1.1](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1.1).
pub const LOCAL_PART: usize = 64;

/// The maximum length of a domain name or number in bytes.
///
/// [RFC 5321 § 4.5.3.1.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1.2).
pub const DOMAIN: usize = 255;

/// The maximum length of a reverse-path or forward-path (including punctuation and separators)
/// in bytes.
///
/// [RFC 5321 § 4.5.3.1.3](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1.3).
pub const PATH: usize = 256;

/// The maximum length of a command line (including the verb and line ending sequence) in
/// bytes.
///
/// [RFC 5321 § 4.5.3.1.4](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1.4).
pub const COMMAND_LINE: usize = 512;

/// The maximum length of a reply line (including the code and line ending sequence) in bytes.
///
/// [RFC 5321 § 4.5.3.1.5](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1.5).
pub const REPLY_LINE: usize = 512;

/// The maximum length of a text line (including the line ending sequence) in bytes.
///
/// [RFC 5321 § 4.5.3.1.6](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1.6).
pub const TEXT_LINE: usize = 1_000;

/// The maximum length of a message (including both the headers and body) in bytes.
///
/// Given the evolution of email, this value is especially recommended to be raised.
///
/// [RFC 5321 § 4.5.3.1.7](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.1.7).
pub const MESSAGE: usize = 64_000;
