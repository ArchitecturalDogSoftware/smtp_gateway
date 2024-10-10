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

//! A collection of the minimum amounts of time that participants in an SMTP session should wait
//! for a reply.
//!
//! Some amount of delays from transmission and processing are expected in an SMTP session. To
//! differentiate between these and a genuinely timed out session, [RFC 5321
//! 4.5.3.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2) defines a list of
//! timeouts in minutes.
//!
//! Notably, [`SERVER_TIMEOUT`] is the only timeout relevant to a server implementation. The
//! timeouts used by clients are here for the sake of testing and thoroughness.
//!
//! Note that, when testing, all timeouts are overridden to [`EXPECTED`]; because a testing
//! environment can be expected to have better performance than the real world.

/// A very strict timeout for how long participants should wait for anything.
///
/// Not specified by RFC 5321. This is for identifying unusual performance for testing and logging.
pub const EXPECTED: std::time::Duration = std::time::Duration::from_secs(3);

/// Generate `const` items with [`std::time::Duration`] values in minutes, optionally including
/// documentation comments.
///
/// Does not account for leap seconds or similar shenanigans. A "minute" is 60 of whatever
/// [`std::time::Duration`] considers to be a "second."
macro_rules! minute_durations {
        [$(
            $( #[$attr:meta] )*
            $label:ident = $minutes:expr
        ),+ ,] => {
            $(
                $( #[$attr] )*
                #[cfg(not(test))]
                pub const $label: ::std::time::Duration =
                    ::std::time::Duration::from_secs($minutes * 60);

                // For stricter performance checks during testing.
                $( #[$attr] )*
                #[cfg(test)]
                pub const $label: ::std::time::Duration =
                    $crate::timeouts::EXPECTED;
            )+
        };
    }

minute_durations![
    /// Servers will sometimes accept TCP connections, but wait for spare processing to initiate
    /// the SMTP session with the opening `220` message. This defines a minimum of how long a
    /// client should wait after the connection is accepted for the `220` reply.
    ///
    /// [RFC 5321 § 4.5.3.2.1](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.1).
    INITIAL_220_MESSAGE = 2,
    /// The minimum length in minutes a client should wait for a reply after sending the `MAIL`
    /// command.
    ///
    /// [RFC 5321 § 4.5.3.2.2](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.2).
    MAIL = 5,
    /// The minimum length in minutes a client should wait for a reply after sending the `RCPT`
    /// command. Mailing lists and aliases take time to process, so this timeout may need to be
    /// longer, depending on when those are processed.
    ///
    /// [RFC 5321 § 4.5.3.2.3](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.3).
    RCPT = 5,
    /// The minimum length in minutes a client should wait for the `354` reply after sending the
    /// `DATA` command.
    ///
    /// [RFC 5321 § 4.5.3.2.4](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.4).
    DATA_INITIALIZATION = 2,
    /// The minimum length in minutes a client should wait for a response after sending a chunk of
    /// data with TCP `send`.
    ///
    /// [RFC 5321 § 4.5.3.2.5](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.5).
    DATA_BLOCK = 3,
    /// The minimum length in minutes a client should wait for the `250` reply after sending
    /// finishing sending all the data. A long delay is necessary here, as the server will need to
    /// process and deliver the message, and prematurely ending it could result in duplicate
    /// messages.
    ///
    /// [RFC 5321 § 4.5.3.2.6](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.6).
    DATA_TERMINATION = 10,
    /// The minimum length in minutes a server should wait for the next command from a client.
    ///
    /// [RFC 5321 § 4.5.3.2.7](https://www.rfc-editor.org/rfc/rfc5321.html#section-4.5.3.2.7).
    SERVER_TIMEOUT = 5,
];
