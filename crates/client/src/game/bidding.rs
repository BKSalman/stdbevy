use bevy::{
    prelude::*,
    ui_widgets::{SliderRange, SliderValue, slider_self_update},
};
use bevy_stdb::prelude::*;
use spacetimedb_sdk::table::TableLike;

use crate::module_bindings::*;
use crate::player::*;
use crate::state::*;
use crate::stdb::*;
use crate::ui::{button, slider, slider_label};
use crate::{
    game::{HandScoreText, PlayerHandRoot},
    ui::text,
};

const BID_STEP: f32 = 1000.0;
const BID_PRECISION: i32 = -3;

#[derive(Component, Debug, Default, Clone)]
pub struct BidSlider;

#[derive(Component, Debug, Default, Clone)]
pub struct BidAmountText;

fn bid_bounds(conn: &StdbConn, game_id: u64, seat_id: u64) -> (f32, f32) {
    let balance = conn
        .db()
        .seat()
        .iter()
        .find(|seat| seat.id == seat_id)
        .map(|seat| seat.balance)
        .unwrap_or_default() as f32;

    let highest = conn
        .db()
        .placed_bid()
        .iter()
        .filter(|bid| bid.game_id == game_id)
        .map(|bid| bid.bidding_amount)
        .max();

    let min = match highest {
        Some(highest) => ((highest as f32 / BID_STEP).floor() + 1.0) * BID_STEP,
        None => BID_STEP,
    };

    let max = (balance / BID_STEP).floor() * BID_STEP;

    (min, max.max(min))
}

#[derive(Component, Debug, Default, Clone)]
pub struct BiddingContainer;

#[derive(Component, Debug, Default, Clone)]
pub struct LocalBidText;

#[derive(Component, Debug, Default, Clone)]
pub struct OnlineBidText;

pub fn spawn_bidding_ui(
    mut commands: Commands,
    conn: Res<StdbConn>,
    current_game: Res<CurrentGame>,
    local_seat: Res<LocalSeat>,
) {
    let (min_bid, max_bid) = bid_bounds(&conn, current_game.id, local_seat.0);
    let hands = conn.db().myhand();
    let hand_score = hands
        .iter()
        .find(|hand| hand.seat_id == local_seat.0)
        .map(|hand| format!("hand score: {}", hand.total_credit))
        .unwrap_or_default();

    commands
        .spawn_scene(bsn! {
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: px(10),
                column_gap: px(10),
            }
            Children [
                HandScoreText
                @text(hand_score)
                TextFont {
                    font_size: FontSize::Px(38.),
                }
                --
                LocalBidText
                TextFont {
                    font_size: FontSize::Px(38.),
                }
                --
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: px(10),
                    column_gap: px(10),
                }
                Children [
                    @slider(min_bid, max_bid, min_bid, BID_STEP, BID_PRECISION)
                    BidSlider
                    on(slider_self_update)
                    on(slider_label::<BidAmountText>)
                    --
                    BidAmountText
                    @text(format!("{}", min_bid as i32))
                    TextFont {
                        font_size: FontSize::Px(38.),
                    }
                    --
                    @button("bid")
                    on(move |mut event: On<PointerPress>, sliders: Query<&SliderValue, With<BidSlider>>, conn: Res<StdbConn>| {
                        if let Ok(value) = sliders.single()
                            && let Err(err) = conn.reducers().place_bid(value.0.round() as i32)
                        {
                            error!("could not request place_bid: {err}");
                        }
                        event.propagate(false);
                    })
                    --
                    @button("fold")
                    on(move |mut event: On<PointerPress>, conn: Res<StdbConn>| {
                        if let Err(err) = conn.reducers().withdraw_from_bidding() {
                            error!("could not request withdraw_from_bidding: {err}");
                        }
                        event.propagate(false);
                    })
                ]
                --
                PlayerHandRoot
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: px(10),
                    column_gap: px(10),
                }
            ]
        })
        .insert(DespawnOnExit(RoundState::Bidding));
}

pub fn handle_placed_bids(
    mut commands: Commands,
    mut bids: ReadInsertUpdateMessage<PlacedBid>,
    current_game: Res<CurrentGame>,
    local_seat_id: Res<LocalSeat>,
    bid_text: Single<Entity, With<LocalBidText>>,
    online_bid_texts: Query<(Entity, &PlayerPosition, &SeatId), With<OnlineBidText>>,
) {
    for msg in bids.read() {
        if msg.new.game_id != current_game.id {
            continue;
        }
        let new_bid = msg
            .old
            .as_ref()
            .is_none_or(|old| old.bidding_amount != msg.new.bidding_amount)
            .then_some(msg.new.bidding_amount);

        if let Some(new_bid) = new_bid {
            if msg.new.seat_id == local_seat_id.0 {
                commands
                    .entity(*bid_text)
                    .insert(Text::new(format!("Your bidding: {}", new_bid)));
            } else if let Some((entity, position, _)) = online_bid_texts
                .iter()
                .find(|(_, _, seat)| seat.0 == msg.new.seat_id)
            {
                commands
                    .entity(entity)
                    .insert(Text::new(format!("{}'s bidding: {}", position.0, new_bid)));
            }
        }
    }
}

pub fn spawn_players_biddings(
    mut commands: Commands,
    conn: Res<StdbConn>,
    current_game: Res<CurrentGame>,
    local_position: Res<LocalPosition>,
) {
    let seat_count = current_game.player_count.max(1);

    let seats = conn.db().seat();

    for seat in seats.iter().filter(|seat| seat.game_id == current_game.id) {
        let player_position = seat.position;
        let relative = (local_position.0 + seat_count - player_position % seat_count) % seat_count;
        let (top, bottom, right, left) = match relative {
            1 => (percent(50.), auto(), auto(), px(0)),
            2 => (px(0), auto(), auto(), percent(50.)),
            3 => (percent(50.), auto(), px(0), auto()),
            _ => (auto(), auto(), auto(), auto()),
        };

        let player_id = seat.player_id;
        let seat_id = seat.id;

        commands
            .spawn_scene(bsn! {
                PlayerMarker(player_id)
                SeatId(seat_id)
                PlayerPosition(player_position)
                Node {
                    position_type: PositionType::Absolute,
                    top,
                    bottom,
                    right,
                    left,
                }
                Children [
                    OnlineBidText
                    SeatId(seat_id)
                    PlayerPosition(player_position)
                ]
            })
            .insert(DespawnOnExit(RoundState::Bidding));
    }
}

pub fn update_bid_range(
    mut commands: Commands,
    conn: Res<StdbConn>,
    current_game: Res<CurrentGame>,
    local_seat: Res<LocalSeat>,
    sliders: Query<(Entity, &SliderRange, &SliderValue), With<BidSlider>>,
) {
    let (min, max) = bid_bounds(&conn, current_game.id, local_seat.0);
    let range = SliderRange::new(min, max);

    for (entity, current, value) in sliders.iter() {
        if *current == range {
            continue;
        }

        commands.entity(entity).insert(range);

        let clamped = range.clamp(value.0);
        if clamped != value.0 {
            commands.entity(entity).insert(SliderValue(clamped));
        }
    }
}

pub struct BiddingPlugin;
impl Plugin for BiddingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(RoundState::Bidding),
            (spawn_bidding_ui, spawn_players_biddings),
        );

        app.add_systems(
            Update,
            handle_placed_bids.run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            Update,
            update_bid_range.run_if(in_state(RoundState::Bidding)),
        );
    }
}
