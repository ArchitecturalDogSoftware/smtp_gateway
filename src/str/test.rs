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
    macro_rules! eq {
        ($str:expr, $expected:expr) => {
            assert_eq!(RawSmtpStr::new($str.as_ascii_str()?).as_str(), $expected)
        };
    }

    eq!("", "");
    eq!("\r", "\r\n");
    eq!("\n", "\r\n");
    eq!("\n\r", "\r\n\r\n");
    eq!("Lorem\r", "Lorem\r\n");
    eq!("Lorem\n", "Lorem\r\n");
    eq!(&"\n".repeat(75), &"\r\n".repeat(75));

    Ok(())
}
