# Independent implementation; exile-leveling data is vendored

poe-overlay targets feature parity with Exile UI's levelling tools, but all code is written from scratch. Exile UI's source may be read to answer "what does it do in case X", but nothing is copied out of it, even though it is MIT-licensed; similarity from solving the same problem is fine. The one deliberate exception is game data: the Route files and gem/quest JSON from exile-leveling (MIT) are vendored with attribution in `THIRD_PARTY.md` and converted into our own Guide format by our own build-time tool.

## Considered Options

- **Exile UI's guide JSON**: rejected; it's in an AHK-oriented format and would be copying from the project we're re-implementing.
- **poedb**: reference only; no licence or API.
