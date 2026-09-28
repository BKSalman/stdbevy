use std::collections::HashMap;

use spacetimedb::rand::seq::SliceRandom;
use spacetimedb::*;

const CARDS_PER_PLAYER: usize = 4;

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Card {
    rank: Rank,
    suit: Suit,
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Ace,
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
    ];
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
}

impl Card {
    /// A fresh, unshuffled 52-card deck.
    pub fn full_deck() -> Vec<Card> {
        Suit::ALL
            .into_iter()
            .flat_map(|suit| Rank::ALL.into_iter().map(move |rank| Card { rank, suit }))
            .collect()
    }
}

// TODO: add `OfflinePlayer` or something instead of deleting the player
#[spacetimedb::table(accessor = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
}

#[derive(SpacetimeType, Debug, Clone, Copy)]
pub enum GameState {
    Lobby,
    Playing,
    Ended,
}

#[spacetimedb::table(accessor = game, public)]
pub struct Game {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub state: GameState,
    pub round_start: u8,
    pub current_seat: u8,
    pub round: u8,
    pub plays_in_round: u8,
    pub player_count: u8,
}

#[derive(Debug)]
#[spacetimedb::table(accessor = seat, public, index(accessor = game_position, btree(columns = [game_id, position])))]
pub struct Seat {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    pub player_id: Identity,
    #[index(btree)]
    pub game_id: u64,
    pub position: u8, // 0..game.player_count, turn order
    pub card_count: u32,
}

#[spacetimedb::table(accessor = player_hand)]
pub struct PlayerHand {
    #[primary_key]
    pub seat_id: u64,
    #[unique]
    pub player_id: Identity,
    #[index(btree)]
    pub game_id: u64,
    pub cards: Vec<Card>,
    pub total_credit: u64,
}

#[spacetimedb::table(accessor = deck)]
pub struct Deck {
    #[primary_key]
    pub game_id: u64,
    pub cards: Vec<Card>,
}

#[spacetimedb::table(accessor = placed_bid, public)]
pub struct PlacedBid {
    #[primary_key]
    pub seat_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub bidding_amount: u64,
}

#[spacetimedb::table(accessor = played_card, public)]
pub struct PlayedCard {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub seat_id: u64,
    pub card: Card,
}

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::debug!("Initializing...");
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    ctx.db
        .player()
        .try_insert(Player {
            identity: ctx.sender(),
        })
        .ok();
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    // TODO: wait for timeout before deleting the player
    if let Some(leaving_seat) = ctx.db.seat().player_id().find(ctx.sender()) {
        if ctx.db.seat().game_id().filter(leaving_seat.game_id).count() == 1 {
            ctx.db.game().id().delete(leaving_seat.game_id);
        }
        #[cfg(feature = "dev")]
        {
            ctx.db.player_hand().game_id().delete(leaving_seat.game_id);
            ctx.db.placed_bid().game_id().delete(leaving_seat.game_id);
            ctx.db.seat().game_id().delete(leaving_seat.game_id);
            ctx.db.game().id().delete(leaving_seat.game_id);
        }
    }
    ctx.db.player_hand().player_id().delete(ctx.sender());
    ctx.db.seat().player_id().delete(ctx.sender());
    ctx.db.player().identity().delete(ctx.sender());
}

#[spacetimedb::reducer]
pub fn create_game(ctx: &ReducerContext) -> Result<(), String> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender()) else {
        return Err(String::from(""));
    };
    let game = ctx.db.game().insert(Game {
        id: 0,
        state: GameState::Lobby,
        round_start: 0,
        plays_in_round: 0,
        current_seat: 0,
        round: 0,
        player_count: 1,
    });

    ctx.db.seat().try_insert(Seat {
        id: 0,
        player_id: player.identity,
        game_id: game.id,
        card_count: 0,
        position: 0,
    })?;

    Ok(())
}

#[spacetimedb::reducer]
pub fn enter_game(ctx: &ReducerContext, game_id: u64) -> Result<(), String> {
    let Some(player) = ctx.db.player().identity().find(ctx.sender()) else {
        return Err(String::from("play not found"));
    };

    let Some(game) = ctx.db.game().id().find(game_id) else {
        return Err(String::from("game not found"));
    };

    let seats = ctx.db.seat().game_id().filter(game_id).count();
    match game.state {
        GameState::Lobby => {
            if seats < 4 {
                ctx.db.seat().try_insert(Seat {
                    id: 0,
                    player_id: player.identity,
                    game_id,
                    card_count: 0,
                    position: seats as u8,
                })?;
            } else {
                return Err(String::from("game is full"));
            }
        }
        GameState::Playing => return Err(String::from("game already started")),
        GameState::Ended => return Err(String::from("game ended")),
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn leave_game(ctx: &ReducerContext) -> Result<(), String> {
    let Some(leaving_seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player is not in a game"));
    };
    let Some(game) = ctx.db.game().id().find(leaving_seat.game_id) else {
        return Err(String::from("game not found"));
    };
    if !matches!(game.state, GameState::Lobby) {
        return Err(String::from("cannot leave a game in progress"));
    }

    let seats: Vec<_> = ctx
        .db
        .seat()
        .game_position()
        .filter((game.id, (leaving_seat.position + 1)..))
        .collect();

    for seat in seats {
        ctx.db.seat().id().update(Seat {
            position: seat.position - 1,
            ..seat
        });
    }
    ctx.db.seat().id().delete(leaving_seat.id);

    if ctx.db.seat().game_id().filter(leaving_seat.game_id).count() == 0 {
        ctx.db.game().id().delete(leaving_seat.game_id);
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn withdraw_from_game(ctx: &ReducerContext) -> Result<(), String> {
    let Some(leaving_seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player is not in a game"));
    };
    let Some(game) = ctx.db.game().id().find(leaving_seat.game_id) else {
        return Err(String::from("game not found"));
    };
    if !matches!(game.state, GameState::Playing) {
        return Err(String::from(
            "cannot withdraw if the game is not in progress",
        ));
    }

    // TODO: handle withdrawal properly here

    ctx.db.seat().game_id().delete(game.id);
    ctx.db.player_hand().game_id().delete(game.id);
    ctx.db.placed_bid().game_id().delete(game.id);
    ctx.db.game().id().delete(game.id);

    Ok(())
}

#[spacetimedb::reducer]
pub fn start_game(ctx: &ReducerContext, game_id: u64) -> Result<(), String> {
    let Some(mut game) = ctx.db.game().id().find(game_id) else {
        return Err(String::from("game not found"));
    };

    if !matches!(game.state, GameState::Lobby) {
        return Err(String::from("game already started"));
    }

    let mut seats: Vec<Seat> = ctx.db.seat().game_id().filter(game.id).collect();
    if seats.len() != 4 {
        return Err(String::from("game needs 4 players to start"));
    }
    seats.sort_by_key(|seat| seat.position);

    let mut cards = Card::full_deck();
    cards.shuffle(&mut ctx.rng());

    for seat in seats {
        let hand = cards.split_off(cards.len() - CARDS_PER_PLAYER);

        let total_credit = calculate_credit(&hand);

        ctx.db.player_hand().insert(PlayerHand {
            seat_id: seat.id,
            player_id: seat.player_id,
            game_id,
            cards: hand,
            total_credit,
        });
        ctx.db.seat().id().update(Seat {
            card_count: CARDS_PER_PLAYER as u32,
            ..seat
        });
    }

    // whatever is left after dealing is the draw pile
    ctx.db.deck().insert(Deck { game_id, cards });

    game.state = GameState::Playing;
    ctx.db.game().id().update(game);

    Ok(())
}

#[spacetimedb::reducer]
pub fn play_card(ctx: &ReducerContext, idx: u32) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(mut game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if game.current_seat != seat.position {
        return Err(String::from("not your turn"));
    }

    let Some(mut player_hand) = ctx.db.player_hand().seat_id().find(seat.id) else {
        return Err(String::from("player doesn't have a playing hand"));
    };

    if idx as usize >= player_hand.cards.len() {
        return Err(String::from("no card with provided index"));
    }

    let card = player_hand.cards.remove(idx as usize);

    ctx.db.played_card().insert(PlayedCard {
        id: 0,
        game_id: seat.game_id,
        seat_id: seat.id,
        card,
    });

    ctx.db.player_hand().seat_id().update(player_hand);

    ctx.db.seat().id().update(Seat {
        card_count: seat.card_count - 1,
        ..seat
    });

    game.current_seat = (game.current_seat + 1) % 4;
    ctx.db.game().id().update(game);

    Ok(())
}

fn calculate_credit(cards: &[Card]) -> u64 {
    let mut total_credit = 0;

    let mut sorted = cards.to_vec();
    sorted.sort_unstable();
    debug_assert!(sorted.len() == CARDS_PER_PLAYER);

    let mut duplicates: HashMap<Rank, Vec<Suit>> = HashMap::from([
        (Rank::Two, Vec::new()),
        (Rank::Three, Vec::new()),
        (Rank::Four, Vec::new()),
        (Rank::Five, Vec::new()),
        (Rank::Six, Vec::new()),
        (Rank::Seven, Vec::new()),
        (Rank::Eight, Vec::new()),
        (Rank::Nine, Vec::new()),
        (Rank::Ten, Vec::new()),
        (Rank::Jack, Vec::new()),
        (Rank::Queen, Vec::new()),
        (Rank::King, Vec::new()),
        (Rank::Ace, Vec::new()),
    ]);

    for card in cards {
        duplicates
            .entry(card.rank)
            .and_modify(|v| v.push(card.suit));
    }

    let is_series = sorted[0].rank as u64 + 1 == sorted[1].rank as u64
        && sorted[1].rank as u64 + 1 == sorted[2].rank as u64
        && sorted[2].rank as u64 + 1 == sorted[3].rank as u64;

    if let Some((r, _s)) = duplicates.iter().find(|(_r, s)| s.len() == 4) {
        total_credit += *r as u64 * 10;
    } else if is_series {
        for card in sorted {
            total_credit += card.rank as u64 * 4;
        }
    } else if let Some((r, _s)) = duplicates.iter().find(|(_r, s)| s.len() == 3) {
        total_credit += *r as u64 * 3;
    } else if duplicates.iter().find(|(_r, s)| s.len() == 2).is_some() {
        let mut iter = duplicates.iter().filter(|(_r, s)| s.len() == 2);

        match (iter.next(), iter.next()) {
            (None, Some((r, _))) | (Some((r, _)), None) => {
                total_credit += *r as u64 * 2;
            }
            (Some((r1, _)), Some((r2, _))) => {
                total_credit += *r1 as u64 * 2;
                total_credit += *r2 as u64 * 2;
            }
            (None, None) => {}
        }
    } else if let Some(max) = cards.iter().map(|c| c.rank as u64).max() {
        total_credit += max;
    }

    total_credit
}

fn next_seat(game: &mut Game) {
    let next_seat = game.current_seat + 1;
    if next_seat >= game.player_count {
        game.round += 1;
        game.current_seat = game.round;
    } else {
        game.current_seat = next_seat;
    }
}

#[spacetimedb::reducer]
pub fn place_bid(ctx: &ReducerContext, bidding_amount: u64) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(mut game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if game.current_seat != seat.position {
        return Err(String::from("not your turn"));
    }

    if let Some(mut placed_bid) = ctx.db.placed_bid().seat_id().find(seat.id) {
        placed_bid.bidding_amount = bidding_amount;
        ctx.db.placed_bid().seat_id().update(placed_bid);
    } else {
        ctx.db.placed_bid().insert(PlacedBid {
            seat_id: seat.id,
            game_id: seat.game_id,
            bidding_amount,
        });
    }

    game.plays_in_round += 1;

    if game.plays_in_round == game.player_count {
        game.round += 1;
        game.round_start = (game.round_start + 1) % game.player_count;
        game.current_seat = game.round_start;
        game.plays_in_round = 0;

        let mut deck = ctx.db.deck().game_id().find(game.id).unwrap();

        let bids: Vec<_> = ctx.db.placed_bid().game_id().filter(game.id).collect();
        // TODO: resolve the bids
        // ...resolve using `bids`...
        for bid in bids {
            ctx.db.placed_bid().seat_id().delete(bid.seat_id);
        }

        // TODO: check if deck.cards.len() > CARDS_PER_PLAYER * seats

        deck.cards.shuffle(&mut ctx.rng());

        for other_seat in ctx.db.seat().game_id().filter(game.id) {
            let hand = deck.cards.split_off(deck.cards.len() - CARDS_PER_PLAYER);

            let total_credit = calculate_credit(&hand);

            let mut player_hand = ctx.db.player_hand().seat_id().find(other_seat.id).unwrap();
            player_hand.cards = hand;
            player_hand.total_credit = total_credit;

            ctx.db.player_hand().seat_id().update(player_hand);
        }

        ctx.db.deck().game_id().update(deck);
    } else {
        game.current_seat = (game.current_seat + 1) % game.player_count;
    }

    ctx.db.game().id().update(game);

    Ok(())
}

#[spacetimedb::reducer]
pub fn withdraw_from_bidding(ctx: &ReducerContext) -> Result<(), String> {
    let Some(seat) = ctx.db.seat().player_id().find(ctx.sender()) else {
        return Err(String::from("player doesn't have a seat"));
    };
    let Some(mut game) = ctx.db.game().id().find(seat.game_id) else {
        return Err(String::from("player is not in a game"));
    };

    if !matches!(game.state, GameState::Playing) {
        return Err(String::from("game is not in progress"));
    }

    if game.current_seat != seat.position {
        return Err(String::from("not your turn"));
    }

    ctx.db.placed_bid().seat_id().delete(seat.id);

    next_seat(&mut game);

    ctx.db.game().id().update(game);

    Ok(())
}

#[spacetimedb::view(accessor = myhand, public)]
pub fn myhand(ctx: &ViewContext) -> Option<PlayerHand> {
    ctx.db.player_hand().player_id().find(ctx.sender())
}

#[cfg(feature = "dev")]
#[spacetimedb::reducer]
pub fn dev_start_solo(ctx: &ReducerContext) -> Result<(), String> {
    if ctx.db.seat().player_id().find(ctx.sender()).is_some() {
        return Err(String::from("already seated"));
    }

    let game = ctx.db.game().insert(Game {
        id: 0,
        state: GameState::Lobby,
        current_seat: 0,
        round_start: 0,
        plays_in_round: 0,
        round: 0,
        player_count: 4,
    });

    ctx.db.seat().try_insert(Seat {
        id: 0,
        player_id: ctx.sender(),
        game_id: game.id,
        position: 0,
        card_count: 0,
    })?;

    for position in 1..4 {
        ctx.db.seat().try_insert(Seat {
            id: 0,
            player_id: Identity::from_claims("dev", &format!("bot{position}")),
            game_id: game.id,
            position,
            card_count: 0,
        })?;
    }

    start_game(ctx, game.id)
}
