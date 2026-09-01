Get a SpacetimeDB Bevy app running in under 5 minutes.

## Prerequisites

- [Rust](https://rust-lang.org/tools/install/) 1.95.0+ installed
- [SpacetimeDB CLI](https://spacetimedb.com/install) installed

---

## Create your project

Run the `spacetime dev` command to create a new project with a SpacetimeDB module and Bevy client.

```bash
spacetime dev --template bevy-rust
```

## Run your app

Once `spacetime dev` has published the module and generated bindings, you can run the client:

```bash
cargo run -p client
```

## Explore the project structure

Your project contains both server and client code.

Edit `crates/server/src/lib.rs` to add tables and reducers. Edit `crates/client/src/main.rs` to build your game logic.

```
my-bevy-app/
├── crates/
│   ├── server/            # Your SpacetimeDB module
│   │   └── src/
│   │       └── lib.rs     # Server-side logic
│   └── client/            # Bevy game client
│       └── src/
│           ├── main.rs    # Game logic and systems
│           └── stdb.rs    # SpacetimeDB connection setup
└── Cargo.toml
```

## Understand tables and reducers

Open `crates/server/src/lib.rs` to see the module code. The template includes a `Player` table and a `move_player` reducer.

Tables store your data. Reducers are functions that modify data — they're the only way to write to the database.

```rust
#[spacetimedb::table(accessor = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub x: f32,
    pub y: f32,
}

#[reducer]
pub fn move_player(ctx: &ReducerContext, x: f32, y: f32) {
    if let Some(mut player) = ctx.db.player().identity().find(ctx.sender()) {
        player.x = x;
        player.y = y;
        ctx.db.player().identity().update(player);
    }
}
```

## Test with the CLI

Open a new terminal and navigate to your project directory. Then use the SpacetimeDB CLI to call reducers and query your data directly.

```bash
# Call the move_player reducer
spacetime call move_player 100.0 200.0

# Query the player table
spacetime sql "SELECT * FROM player"
```

## Next steps

- Read the [Rust SDK Reference](https://spacetimedb.com/docs/intro/core-concepts/clients/rust-reference) for detailed API docs
- Read Bevy [educational materials](https://bevy.org/learn/)
- Check out the [bevy_stdb](https://github.com/onx2/bevy_stdb) readme
- Join the [Bevy](https://discord.gg/7AXzN42yCV) and [SpacetimeDB](https://discord.gg/9XNj5D78E5) discord channels
