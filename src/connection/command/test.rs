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

//! Tests for [`super`].

use ascii::AsAsciiStr;

use super::*;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_command_parsing() -> Result {
    let command = parse("  foo bar baz bim  \r\n".into_ascii_string()?)?;

    // Tests that it constructs the right object.
    assert_eq!(
        command,
        Command {
            line: "  FOO bar baz bim  \r\n".into_ascii_string()?,
            trimmed: 2..17,    // `"FOO bar baz bim"`.
            verb: 2..5,        // "`FOO`".
            text: Some(6..17), // "`bar baz bim`".
            multiline: MultiLine::LastLine,
        }
    );

    // Tests that it produces the right strings.
    assert_eq!(command.line(), "  FOO bar baz bim  \r\n".as_ascii_str()?);
    assert_eq!(command.trimmed(), "FOO bar baz bim".as_ascii_str()?);
    assert_eq!(command.verb(), "FOO".as_ascii_str()?);
    assert_eq!(command.text(), Some("bar baz bim".as_ascii_str()?));

    // Tests that it does not perform any `CRLF` checks.
    assert_eq!(
        parse("foo bar\n".into_ascii_string()?)?.line(),
        "FOO bar\n".as_ascii_str()?
    );

    // Test for handling of no text.
    assert_eq!(
        parse("foo\r\n".into_ascii_string()?)?,
        Command {
            line: "FOO\r\n".into_ascii_string()?,
            trimmed: 0..3,
            verb: 0..3,
            text: None,
            multiline: MultiLine::LastLine,
        }
    );

    // Test that having a space but no text after the verb still counts as no text.
    assert_eq!(
        parse("foo \r\n".into_ascii_string()?)?,
        Command {
            line: "FOO \r\n".into_ascii_string()?,
            trimmed: 0..3,
            verb: 0..3,
            text: None,
            multiline: MultiLine::LastLine,
        }
    );

    Ok(())
}
