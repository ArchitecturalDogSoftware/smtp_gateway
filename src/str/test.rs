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

use super::*;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_raw_smtp_string() -> Result {
    const L: usize = max_lengths::REPLY_LINE;

    macro_rules! eq {
        ($str:expr, $expected:expr) => {
            assert_eq!(RawSmtpStr::<{ L }>::new($str).as_str(), $expected)
        };
    }

    macro_rules! expect_panic {
        ($str:expr, $reason:expr) => {
            match $str.as_ascii_str() {
                Ok(s) => {
                    if std::panic::catch_unwind(|| RawSmtpStr::<{ L }>::new_from_ascii(s)).is_ok() {
                        return Err(
                            concat!("Didn't encounter a panic even though ", $reason).into()
                        );
                    }

                    Ok(())
                }
                Err(e) => Err(e),
            }
        };
    }

    eq!("\r", "\r\n");
    eq!("\n", "\r\n");
    eq!("\n\r", "\r\n\r\n");
    eq!(&"\n".repeat(L / 2), &"\r\n".repeat(L / 2));

    eq!("", "");
    eq!("lorem", "lorem");
    eq!("lorem\r", "lorem\r\n");
    eq!("lorem\n", "lorem\r\n");

    eq!(&" ".repeat(L), " ".repeat(L));

    expect_panic!(
        "\n".repeat(L / 2 + 1),
        "resulting string should be two bytes longer than the buffer allows"
    )?;
    expect_panic!(
        format!("{}\r", "0".repeat(L - 1)),
        "resulting string should be one byte longer than the buffer allows"
    )?;

    Ok(())
}
