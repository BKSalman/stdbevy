use bevy::prelude::*;

use crate::module_bindings::*;
use crate::player::*;
use crate::state::*;
use crate::stdb::*;
use crate::table_list::{TableList, TableListPlugin};
use crate::ui::button;
use crate::ui::text;

#[derive(Component, Debug, Default, Clone)]
pub struct PlayersListRoot;

pub fn spawn_in_lobby_ui(mut commands: Commands, current_game: Res<CurrentGame>) {
    let current_game = current_game.id;
    commands
        .spawn_scene(bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
            }
            Children [
                (
                    button("Start game")
                    on(move |_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().start_game(current_game) {
                            error!("could not request leave_game: {err}");
                        }
                    })
                ),
                (
                    button("Leave game")
                    on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().leave_game() {
                            error!("could not request leave_game: {err}");
                        }
                    })
                )
            ]
        })
        .insert(DespawnOnExit(AppState::InLobby));
    commands
        .spawn_scene(players_list(current_game))
        .insert(DespawnOnExit(AppState::InLobby));
}

pub fn players_list(game_id: u64) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            top: px(80),
            row_gap: px(10),
        }
        Children [
            text(format!("game: #{game_id}")),
            (
                PlayersListRoot
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                }
            ),
        ]
    }
}

impl TableList for Seat {
    type Accessor = SeatTableAccessor;
    type Root = PlayersListRoot;
    type Order = u64;

    fn order(&self) -> u64 {
        self.id
    }

    fn row(self) -> impl Scene {
        let seat_id = self.id;
        bsn! {
            Node {
                flex_direction: FlexDirection::Column,
            }
            Children [
                text(format!("seat: #{seat_id}")),
            ]
        }
    }
}

pub struct LobbyPlugin;
impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TableListPlugin::<Seat>::default());
        app.add_systems(OnEnter(AppState::InLobby), spawn_in_lobby_ui);
    }
}
