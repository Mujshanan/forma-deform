# Smart Controller

**Status:** Directory reserved; implementation not started.

Smart Controller will own general-purpose controller bindings and coordinate input-to-target mappings. It will remain separate from the specialist Form Deform plugin.

Planned responsibilities:

- Define reusable controller bindings with stable IDs.
- Map bone inputs through neutral-centered ranges and easing curves.
- Support absolute and relative output modes.
- Coordinate supported targets, including Form Deform outputs.
- Validate input and target identities before requesting host operations.

The existing `smart-controller-core` crate remains the initial shared/native implementation. This plugin directory will contain the AniCraft command adapter once its workflow and persistence boundary are settled.

Continuous playback/scrubbing evaluation requires a host integration beyond the current command-plugin API.
