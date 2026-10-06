use bevy::prelude::*;

use crate::module_bindings::*;
use crate::stdb::*;
use crate::table_list::{TableList, TableListPlugin};
use crate::ui::button;
use crate::state::*;

#[derive(Component, Debug, Default, Clone)]
pub struct GamesListRoot;

pub fn spawn_main_menu_ui(mut commands: Commands) {
    commands
        .spawn_scene(bsn! {
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    width: percent(100.0),
                    height: percent(100.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                Children [
                    (
                        button("Create a game")
                        on(|_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                            if let Err(err) = conn.reducers().create_game() {
                                error!("could not request create_game: {err}");
                            }
                        })
                    ),
                    (
                        GamesListRoot
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(10),
                        }
                    )
                ]
            )
        })
        .insert(DespawnOnExit(AppState::MainMenu));
}

impl TableList for Game {
    type Accessor = GameTableAccessor;
    type Root = GamesListRoot;
    type Order = u64;

    fn order(&self) -> u64 {
        self.id
    }

    fn row(self) -> impl Scene {
        let game_id = self.id;
        bsn! {
            (
                button(format!("Join game #{game_id}"))
                on(move |_event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                    if let Err(err) = conn.reducers().enter_game(game_id) {
                        error!("could not request enter_game: {err}");
                    }
                })
            )
        }
    }
}

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TableListPlugin::<Game>::default());
        app.add_systems(OnEnter(AppState::MainMenu), spawn_main_menu_ui);
    }
}
