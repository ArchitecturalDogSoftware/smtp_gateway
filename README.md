# smtp_gateway

smtp_gateway is a library for receiving SMTP messages.

## How It Works

`smtp_gateway::listen` accepts any incoming TCP connection and
spawns a new task to handle it as an SMTP session.
When an SMTP session finishes with a received message,
it is passed to the consumer to handle.

smtp_gateway accepts messages but it cannot send or relay messages.
An SMTP gateway receives messages in SMTP and transform them for retransmission.
smtp_gateway exists to handle the first part of this goal,
and it is up to the consumer to handle transformation and retransmission.

For a real example of what this looks like, see smtp_gateway_bot.
This is what smtp_gateway was developed for,
and can be found in [`./smtp_gateway_bot`](./smtp_gateway_bot/).

## Terminology

smtp_gateway uses specific terminology (such as "client" and "server")
as defined by [RFC 5321 section 2.3](https://www.rfc-editor.org/rfc/rfc5321.html#section-2.3).
Pull requests and issues to fix discrepancies are welcome.

## The State of smtp_gateway

smtp_gateway is currently in development,
code quality is not a high priority.
Pull requests and issues are welcome,
especially for pointing out or correcting issues regarding
non-compliance with the SMTP specification.

Code is designed largely just to reach the next part of the specification to follow, not for flexibility.
Once I have finished reading the specification and
the first draft of the implementation,
I will likely rewrite the library
with an eye for quality and maintainability.

## License

smtp_gateway is licensed under the GNU Affero General Public License version 3, or (at your option) any later version.
You should have received a copy of the GNU Affero General Public License along with smtp_gateway, found in [LICENSE](./LICENSE).
If not, see <https://www.gnu.org/licenses/>.
