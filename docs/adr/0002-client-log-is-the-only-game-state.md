# The Client Log is the only automatic source of game state

All automatic tracking (Area changes, levels, deaths, Character detection) comes from tailing `Client.txt`. We never read game memory, inject into or modify the client, or send input to the game, because GGG's third-party policy treats those as bannable. Anything the log cannot tell us (Bandit Choice, Step completion the log misses, Lab Layout) is entered manually or by hotkey, and clipboard reads only happen once per manual key press.

## Consequences

- Features that would "obviously" be automatic (e.g. detecting the passive tree's pan/zoom) need manual Calibration instead.
- The parser must tolerate unknown/changed line formats without crashing, since the log is not a stable API.
