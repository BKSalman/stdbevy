mod cards;
mod module_bindings;
mod stdb;

use bevy::{prelude::*, text::FontSourceTemplate};
use bevy_stdb::prelude::*;
use spacetimedb_sdk::Identity;

use stdb::*;

use crate::module_bindings::{
    Player, create_game, gameQueryTableAccess, myhandQueryTableAccess, played_cardQueryTableAccess,
    playerQueryTableAccess, seatQueryTableAccess,
};

#[derive(Component, Debug, Default)]
pub struct PlayerMarker(Identity);

#[derive(Component, Debug, Default)]
pub struct SeatId(u64);

#[derive(Resource, Debug, Default, Clone)]
pub struct LocalPlayer(Identity);

#[derive(Component, Debug, Default)]
pub struct NetTransform {
    x: f32,
    y: f32,
}

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

        app.add_systems(Startup, spawn_camera);

        app.add_systems(Startup, create_game_ui.spawn());

        app.add_systems(
            PreUpdate,
            (subscribe_on_connect, animate_played_card, despawn_player)
                .run_if(resource_exists::<StdbConn>),
        );
        app.add_systems(
            PreUpdate,
            spawn_player.run_if(resource_exists::<LocalPlayer>),
        );
    }
}

fn button(label: &str) -> impl Scene {
    bsn! {
        Button
        Node {
            width: px(150),
            height: px(65),
            border: px(5),
            border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [(
            Text(label)
            // TextFont {
            //     font: FontSourceTemplate::Handle("fonts/FiraSans-Bold.ttf"),
            //     font_size: px(33.0),
            // }
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
        on(|event: On<Pointer<Enter>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.1))
            );
        })
        on(|event: On<Pointer<Leave>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
            );
        })
        on(|event: On<Pointer<Press>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.2))
            );
        })
        on(|event: On<Pointer<Release>>, mut commands: Commands| {
            commands.entity(event.entity).insert(
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15).lighter(0.1))
            );
        })
    }
}

fn create_game_ui(conn: Res<StdbConn>) -> impl Scene {
    bsn! {
        (
            button("Create a game")
            on(|_event: On<Pointer<Press>>| {
                conn.reducers().create_game().ok();
            })
        )
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn subscribe_on_connect(
    mut commands: Commands,
    mut connected_msgs: ReadStdbConnectedMessage,
    mut subs: ResMut<StdbSubs>,
) {
    for msg in connected_msgs.read() {
        info!("connected as {:?}", msg.identity);
        commands.insert_resource(LocalPlayer(msg.identity));
        subs.subscribe_query(SubKey::Player, |q| q.from.player());
        subs.subscribe_query(SubKey::Seat, |q| q.from.seat());
        subs.subscribe_query(SubKey::Game, |q| q.from.game());
        subs.subscribe_query(SubKey::PlayedCard, |q| q.from.played_card());
        subs.subscribe_query(SubKey::PlayerHand, |q| q.from.myhand());
    }
}

fn spawn_player(
    mut commands: Commands,
    local: Res<LocalPlayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut insert_player_msgs: ReadInsertMessage<module_bindings::Seat>,
) {
    for msg in insert_player_msgs.read() {
        commands.spawn((PlayerMarker(msg.row.player_id), SeatId(msg.row.id)));
    }
}

fn despawn_player(
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

fn animate_played_card(
    mut players: Query<(&SeatId, &mut NetTransform)>,
    mut msgs: ReadUpdateMessage<module_bindings::PlayedCard>,
) {
    for msg in msgs.read() {
        msg.new.seat_id;
        for (seat_id, mut net) in &mut players {
            if seat_id.0 == msg.new.seat_id {
                // TODO: animate
            }
        }
    }
}
