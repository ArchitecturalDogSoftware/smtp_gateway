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

/// Checks whether a string is ASCII and ends with `CRLF`.
///
/// [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321.html) requires that only US-ASCII character
/// encoding (sections 2.3.1 and 2.4) and `CRLF` line endings (section 2.3.8) are used.
#[inline]
pub fn smtp_line(str: &str) -> bool {
    str.ends_with("\r\n") && str.is_ascii()
}

/// Checks if the server's opening message roughly matches [RFC 5321,
/// section 4.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.2).
///
/// Considers a 554 response to be an error.
pub fn server_greeting(str: &str) -> bool {
    str.starts_with("220") && smtp_line(str)
}

pub fn helo(str: &str) -> bool {
    smtp_line(str) && todo!()
}

/// Checks if the server's response to the `QUIT` command matches [RFC 5321, section
/// 4.1.1.10](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.1.1.10).
pub fn quit(str: &str) -> bool {
    smtp_line(str) && str.starts_with("221")
}
