# stdbevy

A SpacetimeDB + Bevy card game.

## Running

```sh
spacetime start                                     # local server
spacetime publish stdbevy-tu6vc --server local -y   # publish the module
cargo run                                           # the client
```

## Dev shortcuts

Debug builds only, and the reducer behind F5 needs the server's `dev` feature (on by default;
publish with `--no-default-features` for a real deployment).

| Key | What it does |
| --- | --- |
| F5  | `dev_start_solo`: creates a game, seats you with three bots, deals — drops you straight into `InGame` |
| F6  | Respawns the current state's UI in place, without leaving the state |

### Hot patching

Edits to system and observer bodies can be patched into the running app, so iterating on UI
doesn't mean replaying the game setup. Needs the [dioxus CLI](https://dioxuslabs.com/learn/0.7/CLI/installation):

```sh
dx serve --hot-patch --features hotpatching -p client
```

Scenes already spawned aren't re-resolved, so pair it with F6 after editing a `spawn_*_ui`
function.

## License

Portions of this project are derived from onx2/spacetimedb-bevy-template, Copyright (c) the template authors, dual-licensed under MIT and Apache-2.0. See licenses/. The work as a whole is licensed under GPLv3.
