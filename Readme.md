# bevy_serde_macros


This crate lets you serialize specified components and their associated entities 
using a "list" of components, as well as the marked entities you wish to serialize.
The result is that the intersection of marked entities and specified components
are serialized. 

Though this currently targets bevy ECS only, other aspects of bevy may be added in the future.

See the tests for usage examples (`save_game` and `load_game`). Currently, the
list of components is specified by a macro that the user must implement
(named `execute_with_type_list` in the examples).

## Acknowledgments

1. The original inspiration was from Herbert "TheBracket" Wolverson's
  [Rust Roguelike Tutorial](https://github.com/amethyst/rustrogueliketutorial)
  (Copyright 2019 Herbert Wolverson (DBA Bracket Productions)).
1. One of the changes to the tutorial was largely thanks to the addition of `execute_with_type_list!`,
  with credit to Michael F. Bryan - see code comments for details.
1. The code from the tutorial relied on some
  [helper functions from specs](https://docs.rs/specs/latest/src/specs/saveload/ser.rs.html#37-59)
  (Copyright (c) 2017 The Specs Project Developers), which I had to rewrite for Bevy.
1. Also thanks to the Bevy community for getting me up to speed on the Bevy ECS.