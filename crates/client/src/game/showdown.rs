use bevy::prelude::*;

use crate::{state::*, ui::text};

pub fn spawn_showdown_ui(mut commands: Commands, round_winner: Res<RoundWinner>) {
    let round_winner = round_winner.0;
    commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                left: px(0),
                top: px(0),
                bottom: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
            }
            Children [
                @text(format!("{} فاز بالشوط!", round_winner))
            ]
        })
        .insert(DespawnOnExit(RoundState::Showdown));
}

pub struct ShowdownPlugin;
impl Plugin for ShowdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RoundState::Showdown), spawn_showdown_ui);
    }
}
