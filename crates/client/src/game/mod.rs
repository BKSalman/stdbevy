pub mod bidding;
pub mod negotiation;
pub mod showdown;

use bevy::prelude::*;
use bevy_stdb::prelude::*;

use crate::game::bidding::{LocalBidText, OnlineBidText};
use crate::module_bindings::*;
use crate::player::*;
use crate::state::*;
use crate::stdb::*;
use crate::table_list::{TableList, ViewListPlugin};
use crate::ui::{button, text};

use crate::game::{
    bidding::BiddingPlugin, negotiation::NegotiationPlugin, showdown::ShowdownPlugin,
};

#[derive(Component, Debug, Default, Clone)]
pub struct PlayerHandRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct CurrentTurnRoot;

#[derive(Component, Debug, Default, Clone)]
pub struct HandScoreText;

pub fn spawn_in_game_ui(
    mut commands: Commands,
    current_game: Res<CurrentGame>,
    local_seat: Res<LocalSeat>,
) {
    let game = current_game.id;
    let local_seat = local_seat.0;
    commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: px(5),
                top: px(5),
            }
            Children [
                (
                    text(format!("current game: {}", game))
                    text(format!("your seat: {}", local_seat))
                ),
                (
                    button("withdraw")
                    on(move |mut event: On<Pointer<Press>>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().withdraw_from_game() {
                            error!("could not request withdraw_from_game: {err}");
                        }
                        event.propagate(false);
                    })
                ),
                (
                    CurrentTurnRoot
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(5),
                        top: px(50),
                    }
                    text(format!("current turn: {}", 0))
                )
            ]
        })
        .insert(DespawnOnExit(AppState::InGame));
}

impl TableList for PlayerHand {
    type Accessor = MyhandTableAccessor;
    type Root = PlayerHandRoot;
    type Order = u64;

    fn order(&self) -> u64 {
        self.seat_id
    }

    fn row(self) -> impl Scene {
        let cards_scenes: Vec<_> = self
            .cards
            .iter()
            .map(|c| {
                let card_name = format!("{:?} {:?}", c.rank, c.suit);

                bsn! {
                    Node {
                        border: px(2),
                    }
                    BorderColor::from(Color::BLACK)
                    text(card_name)
                }
            })
            .collect();

        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: px(10),
                row_gap: px(10),
            }
            Children [
                { cards_scenes }
            ]
        }
    }
}

pub fn handle_new_round(
    mut commands: Commands,
    current_game: Res<CurrentGame>,
    mut games: ReadUpdateMessage<Game>,
    online_bids: Query<Entity, With<OnlineBidText>>,
    local_bid: Single<Entity, With<LocalBidText>>,
) {
    for msg in games.read() {
        if msg.new.id != current_game.id {
            continue;
        }

        if msg.old.round != msg.new.round {
            commands.entity(*local_bid).insert(Text::default());
            for online_bid in online_bids {
                commands.entity(online_bid).insert(Text::default());
            }
        }
    }
}

pub fn update_turn_text(
    mut games: ReadUpdateMessage<Game>,
    mut text: Single<&mut Text, With<CurrentTurnRoot>>,
) {
    for msg in games.read() {
        text.0 = format!("current turn: {}", msg.new.current_seat);
    }
}

pub fn update_hand_score_text(
    mut player_hand: ReadInsertMessage<PlayerHand>,
    local_seat: Res<LocalSeat>,
    mut commands: Commands,
    hand_score_text: Single<Entity, With<HandScoreText>>,
) {
    for msg in player_hand.read() {
        if msg.row.seat_id != local_seat.0 {
            continue;
        }

        commands
            .entity(*hand_score_text)
            .insert(Text::new(format!("hand score: {}", msg.row.total_credit)));
    }
}

pub struct GamePlugin;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ViewListPlugin::<PlayerHand>::default());
        app.add_plugins((BiddingPlugin, NegotiationPlugin, ShowdownPlugin));

        app.add_systems(OnEnter(AppState::InGame), spawn_in_game_ui);

        app.add_systems(
            Update,
            (update_turn_text, update_hand_score_text, handle_new_round)
                .run_if(in_state(AppState::InGame)),
        );
    }
}
