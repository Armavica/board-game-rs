use rand::Rng;

use crate::ai::Bot;
use crate::board::Board;
use internal_iterator::InternalIterator;

pub struct RandomBot<R: Rng> {
    rng: R,
}

impl<R: Rng> RandomBot<R> {
    pub fn new(rng: R) -> Self {
        RandomBot { rng }
    }
}

impl<B: Board, R: Rng> Bot<B> for RandomBot<R> {
    fn select_move(&mut self, board: &B) -> B::Move {
        board.random_available_move(&mut self.rng)
    }
}

pub struct RolloutBot<R: Rng> {
    rollouts_per_move: u32,
    rng: R,
}

impl<R: Rng> RolloutBot<R> {
    pub fn new(rollouts_per_move: u32, rng: R) -> Self {
        RolloutBot { rollouts_per_move, rng }
    }
}

impl<B: Board, R: Rng> Bot<B> for RolloutBot<R> {
    fn select_move(&mut self, board: &B) -> B::Move {
        board.available_moves().max_by_key(|&mv| {
            let child = board.clone_and_play(mv);

            let score: i64 = (0..self.rollouts_per_move).map(|_| {
                let mut copy = child.clone();
                while !copy.is_done() {
                    copy.play(copy.random_available_move(&mut self.rng))
                }
                copy.outcome().unwrap().sign(board.next_player()) as i64
            }).sum();

            score
        }).unwrap()
    }
}