//! Generic "one table row, one list child" UI syncing.
//!
//! Implement [`TableList`] for a generated row type and add [`TableListPlugin<Row>`];
//! the children of the entity marked with `TableList::Root` then follow the rows of
//! that table, with no per-table system to write.

use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_stdb::prelude::*;
use spacetimedb_sdk::{
    // Only reachable through `__codegen`, same as `bevy_stdb` does internally.
    __codegen::{AbstractEventContext, InModule, SpacetimeModule},
    TableAccessor,
    table::TableLike,
};

use crate::{module_bindings::RemoteTables, stdb::StdbConn};

/// The event SpacetimeDB attaches to row callbacks for `T`. `bevy_stdb`'s row messages
/// only exist when it is `Send + Sync`, so it shows up in the bounds below.
type RowEvent<T> =
    <<<T as InModule>::Module as SpacetimeModule>::EventContext as AbstractEventContext>::Event;

/// A table whose rows are rendered as the children of a list container.
pub trait TableList: InModule + Clone + Send + Sync + 'static {
    /// The generated accessor for the table, e.g. `GameTableAccessor`.
    type Accessor: TableAccessor<RemoteTables, Row = Self> + Send + Sync + 'static;

    /// Marker component on the container the rows are spawned into.
    type Root: Component;

    /// Rows are rendered in ascending order of this key. SpacetimeDB makes no promise
    /// about iteration order, so without it the list shuffles every time it is rebuilt.
    type Order: Ord;

    /// The sort key of this row.
    fn order(&self) -> Self::Order;

    /// The scene for a single row.
    fn row(self) -> impl Scene;
}

/// Keeps the children of the `T::Root` entity in sync with the rows of `T`'s table.
pub struct TableListPlugin<T: TableList>(PhantomData<fn() -> T>);

impl<T: TableList> Default for TableListPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> Plugin for TableListPlugin<T>
where
    T: TableList,
    RowEvent<T>: Send + Sync,
    for<'db> <T::Accessor as TableAccessor<RemoteTables>>::Handle<'db>: TableLike<Row = T>,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            sync_table_list::<T>.run_if(resource_exists::<StdbConn>),
        );
    }
}

/// Rebuilds the list when its rows changed, or when the container was just spawned
/// (re-entering a state gives a fresh, empty container that no row message follows).
fn sync_table_list<T>(
    mut commands: Commands,
    conn: Res<StdbConn>,
    root: Single<Entity, With<T::Root>>,
    new_root: Query<(), Added<T::Root>>,
    mut inserts: ReadInsertUpdateMessage<T>,
    mut deletes: ReadDeleteMessage<T>,
) where
    T: TableList,
    RowEvent<T>: Send + Sync,
    for<'db> <T::Accessor as TableAccessor<RemoteTables>>::Handle<'db>: TableLike<Row = T>,
{
    // `+` rather than `||` so both readers are always drained
    let row_changed = inserts.read().count() + deletes.read().count() > 0;
    if !row_changed && new_root.is_empty() {
        return;
    }

    let mut rows: Vec<T> = T::Accessor::get(conn.db()).iter().collect();
    rows.sort_by(|a, b| a.order().cmp(&b.order()));

    commands.entity(*root).despawn_children();
    for row in rows {
        commands.spawn_scene(row.row()).insert(ChildOf(*root));
    }
}
