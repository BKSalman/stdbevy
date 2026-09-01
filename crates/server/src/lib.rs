use std::time::Duration;

use spacetimedb::*;

use crate::math::DbVector2;

mod math {
    use spacetimedb::SpacetimeType;

    // This allows us to store 2D points in tables.
    #[derive(SpacetimeType, Debug, Clone, Copy)]
    pub struct DbVector2 {
        pub x: f32,
        pub y: f32,
    }

    impl std::ops::Add<&DbVector2> for DbVector2 {
        type Output = DbVector2;

        fn add(self, other: &DbVector2) -> DbVector2 {
            DbVector2 {
                x: self.x + other.x,
                y: self.y + other.y,
            }
        }
    }

    impl std::ops::Add<DbVector2> for DbVector2 {
        type Output = DbVector2;

        fn add(self, other: DbVector2) -> DbVector2 {
            DbVector2 {
                x: self.x + other.x,
                y: self.y + other.y,
            }
        }
    }

    impl std::ops::AddAssign<DbVector2> for DbVector2 {
        fn add_assign(&mut self, rhs: DbVector2) {
            self.x += rhs.x;
            self.y += rhs.y;
        }
    }

    impl std::iter::Sum<DbVector2> for DbVector2 {
        fn sum<I: Iterator<Item = DbVector2>>(iter: I) -> Self {
            let mut r = DbVector2::new(0.0, 0.0);
            for val in iter {
                r += val;
            }
            r
        }
    }

    impl std::ops::Sub<&DbVector2> for DbVector2 {
        type Output = DbVector2;

        fn sub(self, other: &DbVector2) -> DbVector2 {
            DbVector2 {
                x: self.x - other.x,
                y: self.y - other.y,
            }
        }
    }

    impl std::ops::Sub<DbVector2> for DbVector2 {
        type Output = DbVector2;

        fn sub(self, other: DbVector2) -> DbVector2 {
            DbVector2 {
                x: self.x - other.x,
                y: self.y - other.y,
            }
        }
    }

    impl std::ops::SubAssign<DbVector2> for DbVector2 {
        fn sub_assign(&mut self, rhs: DbVector2) {
            self.x -= rhs.x;
            self.y -= rhs.y;
        }
    }

    impl std::ops::Mul<f32> for DbVector2 {
        type Output = DbVector2;

        fn mul(self, other: f32) -> DbVector2 {
            DbVector2 {
                x: self.x * other,
                y: self.y * other,
            }
        }
    }

    impl std::ops::Div<f32> for DbVector2 {
        type Output = DbVector2;

        fn div(self, other: f32) -> DbVector2 {
            if other != 0.0 {
                DbVector2 {
                    x: self.x / other,
                    y: self.y / other,
                }
            } else {
                DbVector2 { x: 0.0, y: 0.0 }
            }
        }
    }

    impl DbVector2 {
        pub fn new(x: f32, y: f32) -> Self {
            Self { x, y }
        }

        pub fn sqr_magnitude(&self) -> f32 {
            self.x * self.x + self.y * self.y
        }

        pub fn magnitude(&self) -> f32 {
            (self.x * self.x + self.y * self.y).sqrt()
        }

        pub fn normalized(self) -> DbVector2 {
            self / self.magnitude()
        }
    }
}

#[spacetimedb::table(accessor = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
}

#[spacetimedb::table(accessor = circle, public)]
#[derive(Debug)]
pub struct Circle {
    #[primary_key]
    pub player_id: Identity,
    pub position: DbVector2,
    pub direction: DbVector2,
    pub speed: f32,
}

#[spacetimedb::table(accessor = move_all_players_timer, scheduled(move_all_players))]
pub struct MoveAllPlayersTimer {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}

const SPEED: f32 = 50.0;

#[spacetimedb::reducer]
pub fn move_all_players(ctx: &ReducerContext, _timer: MoveAllPlayersTimer) -> Result<(), String> {
    for mut circle in ctx.db.circle().iter() {
        let vel = circle.direction * circle.speed;
        let new_pos = circle.position + vel * SPEED;
        circle.position.x = new_pos.x;
        circle.position.y = new_pos.y;
        ctx.db.circle().player_id().update(circle);
    }

    Ok(())
}

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::debug!("Initializing...");

    ctx.db
        .move_all_players_timer()
        .try_insert(MoveAllPlayersTimer {
            scheduled_id: 0,
            scheduled_at: ScheduleAt::Interval(Duration::from_millis(50).into()),
        })?;
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    let player = ctx.db.player().insert(Player {
        identity: ctx.sender(),
    });

    ctx.db.circle().insert(Circle {
        player_id: player.identity,
        position: DbVector2::new(0., 0.),
        direction: DbVector2::new(0., 0.),
        speed: 0.,
    });
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    ctx.db.player().identity().delete(ctx.sender());
}

#[reducer]
pub fn set_direction(ctx: &ReducerContext, direction: DbVector2) {
    if let Some(mut circle) = ctx.db.circle().player_id().find(ctx.sender()) {
        circle.direction = direction.normalized();
        circle.speed = direction.magnitude().clamp(0., 1.);
        ctx.db.circle().player_id().update(circle);
    }
}
