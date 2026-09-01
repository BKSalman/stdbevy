mod module_bindings;
mod stdb;

use bevy::prelude::*;
use bevy_stdb::prelude::*;
// use module_bindings::*;
use stdb::*;

use crate::module_bindings::{
    DbVector2, Player, circleQueryTableAccess, playerQueryTableAccess, set_direction,
};

#[derive(Component, Debug, Default)]
pub struct PlayerMarker;

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

        app.add_systems(Startup, (spawn_camera, helper_text.spawn()));

        app.add_systems(PreUpdate, subscribe_on_connect);
        app.add_systems(
            PreUpdate,
            (subscribe_on_connect, sync_position, spawn_player).run_if(resource_exists::<StdbConn>),
        );
        app.add_systems(
            Update,
            (interpolate, handle_move_request).run_if(resource_exists::<StdbConn>),
        );
    }
}

fn helper_text() -> impl Scene {
    bsn! {
        Text::new("Use WASD to move.")
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            left: px(16),
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn subscribe_on_connect(mut connected_msgs: ReadStdbConnectedMessage, mut subs: ResMut<StdbSubs>) {
    for msg in connected_msgs.read() {
        subs.subscribe_query(SubKey::Circle, |q| {
            q.from.circle().r#where(|c| c.player_id.eq(msg.identity))
        });
        subs.subscribe_query(SubKey::Player, |q| {
            q.from.player().r#where(|p| p.identity.eq(msg.identity))
        });
    }
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut insert_player_msgs: ReadInsertMessage<module_bindings::Circle>,
) {
    for msg in insert_player_msgs.read() {
        commands.spawn((
            PlayerMarker,
            Mesh2d(meshes.add(Circle::new(20.0))),
            MeshMaterial2d(materials.add(Color::srgb(0.2, 0.4, 1.0))),
            Transform::from_xyz(msg.row.position.x, msg.row.position.y, 0.0),
            NetTransform {
                x: msg.row.position.x,
                y: msg.row.position.y,
            },
        ));
    }
}

/// Interpolate the rendered position of the player toward the server authority's position
///
/// NOTE: Single will silently fail if there are more than one of the type found.
fn interpolate(
    time: Res<Time>,
    player: Single<(&mut Transform, &NetTransform), With<PlayerMarker>>,
    window: Single<&Window>,
) {
    let dt = time.delta_secs();
    let (mut transform, net_transform) = player.into_inner();
    let target = Vec3::new(net_transform.x, net_transform.y, transform.translation.z);

    let distance = transform.translation.distance(target);

    // If the distance is larger than half the screen width, assume screen edge wrapping.
    let wrap_threshold = window.width() / 2.0;
    if distance > wrap_threshold {
        transform.translation = target;
    } else {
        transform.translation.smooth_nudge(&target, 18.0, dt);
    }
}

/// Store the server authority position on the Player component for use in interpolate system
fn sync_position(
    mut player: Single<&mut NetTransform, With<PlayerMarker>>,
    mut msgs: ReadUpdateMessage<module_bindings::Circle>,
) {
    for msg in msgs.read() {
        player.x = msg.new.position.x;
        player.y = msg.new.position.y;
    }
}

fn handle_move_request(conn: Res<StdbConn>, keys: Res<ButtonInput<KeyCode>>) {
    let mut direction = Vec2::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    let direction = DbVector2 {
        x: direction.x,
        y: direction.y,
    };

    let _ = conn.reducers().set_direction(direction);
}
