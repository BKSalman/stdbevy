mod camera;
mod dev;
mod game;
mod lobby;
mod menu;
mod module_bindings;
mod net;
mod player;
mod state;
mod stdb;
mod table_list;
mod ui;

use bevy::prelude::*;

use crate::{
    camera::CameraPlugin, dev::DevPlugin, game::GamePlugin, lobby::LobbyPlugin, menu::MenuPlugin,
    net::NetPlugin, player::PlayerPlugin, state::StatePlugin, stdb::MyStdbPlugin,
};

fn main() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;
impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Window {
                    title: String::from("SpacetimeDB + Bevy template"),
                    fit_canvas_to_parent: true,
                    ..default()
                }
                .into(),
                ..default()
            }),
        );

        app.add_plugins(MyStdbPlugin);

        app.add_plugins((
            StatePlugin,
            CameraPlugin,
            PlayerPlugin,
            NetPlugin,
            MenuPlugin,
            LobbyPlugin,
            GamePlugin,
            DevPlugin,
        ));
    }
}
