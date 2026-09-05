use spacetimedb::rand::seq::SliceRandom;
use spacetimedb::*;

const CARDS_PER_PLAYER: usize = 5;

#[derive(SpacetimeType, Debug, Clone, Copy)]
pub enum Rank {
    Ace,
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
}

#[derive(SpacetimeType, Debug, Clone, Copy)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(SpacetimeType, Debug, Clone, Copy)]
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
    pub current_seat: u8,
}

#[spacetimedb::table(accessor = seat, public, index(accessor = game_position, btree(columns = [game_id, position])))]
pub struct Seat {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    pub player_id: Identity,
    #[index(btree)]
    pub game_id: u64,
    pub position: u8, // 0..3, turn order
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
}

#[spacetimedb::table(accessor = deck)]
pub struct Deck {
    #[primary_key]
    pub game_id: u64,
    pub cards: Vec<Card>,
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
    ctx.db.player().identity().delete(ctx.sender());
    ctx.db.seat().player_id().delete(ctx.sender());
    ctx.db.player_hand().player_id().delete(ctx.sender());
}

#[spacetimedb::reducer]
pub fn create_game(ctx: &ReducerContext) -> Result<(), String> {
    if let Some(player) = ctx.db.player().identity().find(ctx.sender()) {
        let game = ctx.db.game().insert(Game {
            id: 0,
            state: GameState::Lobby,
            current_seat: 0,
        });

        ctx.db.seat().try_insert(Seat {
            id: 0,
            player_id: player.identity,
            game_id: game.id,
            card_count: 0,
            position: 0,
        })?;
    }

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

        ctx.db.player_hand().insert(PlayerHand {
            seat_id: seat.id,
            player_id: seat.player_id,
            game_id,
            cards: hand,
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

#[spacetimedb::view(accessor = myhand, public)]
pub fn myhand(ctx: &ViewContext) -> Option<PlayerHand> {
    ctx.db.player_hand().player_id().find(ctx.sender())
}
