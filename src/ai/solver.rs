use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::ai::Bot;
use crate::ai::minimax::{Heuristic, minimax, minimax_value};
use crate::board::{Board, Outcome, Player};
use crate::wdl::POV;

/// Heuristic with `bound()-length` for win, `-bound()+length` for loss and 0 for draw.
/// This means the sign of the final minimax value means forced win, forced loss or unknown, and the selected move is
/// the shortest win of the longest loss.
#[derive(Debug)]
pub struct SolverHeuristic;

impl<B: Board> Heuristic<B> for SolverHeuristic {
    type V = i32;

    fn bound(&self) -> Self::V {
        i32::MAX
    }

    fn value(&self, board: &B, length: u32) -> i32 {
        board.outcome().map_or(0, |p| {
            p.pov(board.next_player()).sign::<i32>() * (i32::MAX - length as i32)
        })
    }
}

/// Return which player can force a win if any. Both forced draws and unknown results are returned as `None`.
pub fn find_forcing_winner(board: &impl Board, depth: u32) -> Option<Player> {
    let value = minimax_value(board, &SolverHeuristic, depth);
    match value.cmp(&0) {
        Ordering::Less => Some(board.next_player().other()),
        Ordering::Equal => None,
        Ordering::Greater => Some(board.next_player()),
    }
}

/// Return whether this board is a double forced draw, ie. no matter what either player does the game can only end in a draw.
/// Returns `None` if the result is unknown.
pub fn is_double_forced_draw(board: &impl Board, depth: u32) -> Option<bool> {
    if let Some(outcome) = board.outcome() {
        return Some(outcome == Outcome::Draw);
    }
    if depth == 0 { return None; }

    //TODO this Some/None mapping is super confusing, maybe add try_fold to internal_iterator and use that
    let result = board.available_moves().find_map(|mv| {
        let child = board.clone_and_play(mv);

        match is_double_forced_draw(&child, depth - 1) {
            Some(true) => None,
            Some(false) => Some(false),
            None => Some(true),
        }
    });

    match result {
        Some(true) => None,
        Some(false) => Some(false),
        None => Some(true)
    }
}

pub struct SolverBot<R: Rng> {
    depth: u32,
    rng: R,
}

impl<R: Rng> Debug for SolverBot<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SolverBot {{ depth: {} }}", self.depth)
    }
}

impl<R: Rng> SolverBot<R> {
    pub fn new(depth: u32, rng: R) -> Self {
        assert!(depth > 0);
        SolverBot { depth, rng }
    }
}

impl<B: Board, R: Rng> Bot<B> for SolverBot<R> {
    fn select_move(&mut self, board: &B) -> B::Move {
        minimax(board, &SolverHeuristic, self.depth, &mut self.rng).best_move.unwrap()
    }
}
