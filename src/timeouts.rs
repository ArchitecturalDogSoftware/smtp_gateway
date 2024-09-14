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

//! A collection of the minimum minutes that participant in an SMTP session should wait for a given
//! action.
//!
//! Per [RFC 5321 4.5.3.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2).

/// Generate `const` items with [`std::time::Duration`] values in minutes, optionally including
/// documentation comments.
///
/// Does not account for leap seconds or similar shenanigans, it is exclusively 60 seconds per
/// minute.
macro_rules! minute_durations {
        [$(
            $( #[doc = $docs:expr] )*
            $label:ident = $minutes:expr
        ),+ ,] => {
            $(
                $( #[doc = $docs] )*
                pub const $label: ::std::time::Duration =
                    ::std::time::Duration::from_secs($minutes * 60);
            )+
        };
    }

minute_durations![
    /// [RFC 5231 § 4.5.3.2.1](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.1).
    INITIAL_220_MESSAGE = 2,
    /// [RFC 5231 § 4.5.3.2.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.2).
    MAIL = 5,
    /// [RFC 5231 § 4.5.3.2.3](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.3).
    RCPT = 5,
    /// [RFC 5231 § 4.5.3.2.4](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.4).
    DATA_INITIALIZATION = 2,
    /// [RFC 5231 § 4.5.3.2.5](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.5).
    DATA_BLOCK = 3,
    /// [RFC 5231 § 4.5.3.2.6](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.6).
    DATA_TERMINATION = 10,
    /// [RFC 5231 § 4.5.3.2.7](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.7).
    SERVER_TIMEOUT = 5,
];
