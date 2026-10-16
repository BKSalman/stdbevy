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
touch crates/client/src/main.rs && dx serve --hot-patch -p client --features hotpatching
```

The `touch` is not optional. `dx` learns how to link a patch by intercepting rustc, so the client
has to actually go through a link on startup; if cargo finds the binary fresh it skips the link and
`dx` fails immediately with `Missing linker args for fat link of 'client'`. Touching the crate root
forces the relink, and costs ~10s. Run only one `dx serve` at a time, too — several of them share
the target dir, so whichever loses the race finds everything fresh and fails the same way.

`dx` builds under its own `desktop-dev` profile, so the first run is a full rebuild; after that
a patch lands in a few seconds without restarting the app or dropping the SpacetimeDB connection.

Already-spawned scenes aren't re-resolved by the patch, so every patch also triggers F6's
respawn (`respawn_state_ui`): the current `AppState`, `RoundState` and `PauseMenu` UI is
despawned and its `OnEnter` systems run again, which is what makes an edit to a `spawn_*_ui`
function show up. Game state lives in the database, so it survives the respawn.

## License

Portions of this project are derived from onx2/spacetimedb-bevy-template, Copyright (c) the template authors, dual-licensed under MIT and Apache-2.0. See licenses/. The work as a whole is licensed under GPLv3.
