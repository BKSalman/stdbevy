use bevy::prelude::*;
use bevy_stdb::prelude::*;
use spacetimedb_sdk::Identity;

use crate::module_bindings::Player;
use crate::stdb::StdbConn;

#[derive(Component, Debug, Default, Clone)]
pub struct PlayerMarker(pub Identity);

#[cfg_attr(feature = "inspector", derive(Reflect))]
#[derive(Component, Debug, Default, Clone)]
pub struct SeatId(pub u64);

#[cfg_attr(feature = "inspector", derive(Reflect))]
#[derive(Component, Debug, Default, Clone)]
pub struct PlayerPosition(pub u8);

#[derive(Resource, Debug, Default, Clone)]
pub struct LocalPlayer(pub Identity);

#[cfg_attr(feature = "inspector", derive(Reflect))]
#[derive(Resource, Debug, Default)]
pub struct LocalSeat(pub u64);

#[cfg_attr(feature = "inspector", derive(Reflect))]
#[derive(Resource, Debug, Default)]
pub struct LocalPosition(pub u8);

#[cfg_attr(feature = "inspector", derive(Reflect))]
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct CurrentGame {
    pub id: u64,
    pub player_count: u8,
}

pub fn despawn_player(
    mut commands: Commands,
    players: Query<(Entity, &PlayerMarker)>,
    mut delete_msgs: ReadDeleteMessage<Player>,
) {
    for msg in delete_msgs.read() {
        for (entity, marker) in &players {
            if marker.0 == msg.row.identity {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, despawn_player.run_if(resource_exists::<StdbConn>));
    }
}
