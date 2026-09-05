use crate::module_bindings::*;
use bevy::prelude::*;
use bevy_stdb::prelude::*;

// You can define domain-specific subscription keys when managing subscriptions via bevy_stdb,
// or delete this and `.with_subscriptions` to manage yourself through the raw connection resource.
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum SubKey {
    Player,
    Seat,
    Game,
    Deck,
    PlayerHand,
    PlayedCard,
}

// The SpacetimeDB connection resource
// You can use like `conn: Res<StdbConn>`
pub type StdbConn = StdbConnection<DbConnection>;

// bevy_stdb's Command API for connect, disconnect, setting token
// You can use like `mut stdb_cmds: StdbCmds`
// pub type StdbCmds<'w, 's> = StdbCommands<'w, 's, DbConnection, RemoteModule>;

// Alias for the bevy_stdb subscription mechanism, keyed by your enum `SubKey`
// You can use like: `mut subs: ResMut<StdbSubs>`
pub type StdbSubs = StdbSubscriptions<SubKey, RemoteModule>;

pub struct MyStdbPlugin;
impl Plugin for MyStdbPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            StdbPlugin::<DbConnection, RemoteModule>::default()
                .with_uri(String::from("http://localhost:3001"))
                .with_database_name(String::from("stdbevy-tu6vc"))
                // Enables subscription management within bevy_stdb
                .with_subscriptions::<SubKey>()
                // Connects directly upon plugin build
                .with_eager_connection()
                // You can register tables, views, and tables without PK using methods like this:
                // add_view, add_table, add_table_without_pk
                .add_table::<PlayerTableAccessor>()
                .add_table::<SeatTableAccessor>()
                .add_table::<PlayedCardTableAccessor>()
                .add_view::<MyhandTableAccessor>()
                // Typical case is using a background native driver, but there are others available for web or frame-driven
                .with_background_driver(DbConnection::run_threaded),
        );
    }
}
