use std::cmp::Ordering;

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::board::{Board, BoardAvailableMoves, Outcome, Player};
use crate::games::ataxx::{Coord, Move, Tiles};
use crate::symmetry::D4Symmetry;

const MAX_MOVES_SINCE_LAST_COPY: u8 = 100;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct AtaxxBoard {
    pub(super) tiles_a: Tiles,
    pub(super) tiles_b: Tiles,
    pub(super) gaps: Tiles,
    pub(super) moves_since_last_copy: u8,
    pub(super) next_player: Player,
    pub(super) outcome: Option<Outcome>,
}

impl Default for AtaxxBoard {
    fn default() -> Self {
        AtaxxBoard {
            tiles_a: Tiles::CORNERS_A,
            tiles_b: Tiles::CORNERS_B,
            gaps: Tiles::empty(),
            moves_since_last_copy: 0,
            next_player: Player::A,
            outcome: None,
        }
    }
}

impl AtaxxBoard {
    pub fn empty() -> Self {
        AtaxxBoard {
            tiles_a: Tiles::empty(),
            tiles_b: Tiles::empty(),
            gaps: Tiles::empty(),
            moves_since_last_copy: 0,
            next_player: Player::A,
            outcome: Some(Outcome::Draw),
        }
    }

    pub fn tile(&self, coord: Coord) -> Option<Player> {
        if self.tiles_a.has(coord) {
            return Some(Player::A);
        }
        if self.tiles_b.has(coord) {
            return Some(Player::B);
        }
        None
    }

    pub fn tiles_a(&self) -> Tiles {
        self.tiles_a
    }

    pub fn tiles_b(&self) -> Tiles {
        self.tiles_b
    }

    pub fn gaps(&self) -> Tiles {
        self.gaps
    }

    pub fn free_tiles(&self) -> Tiles {
        !(self.tiles_a | self.tiles_b | self.gaps)
    }

    /// Return whether the player with the given tiles has to pass, ie. cannot make a copy or jump move.
    fn must_pass(&self, tiles: Tiles) -> bool {
        let possible_targets = tiles.copy_targets() | tiles.jump_targets();
        (possible_targets & self.free_tiles()).is_empty()
    }

    pub fn tiles_pov(&self) -> (Tiles, Tiles) {
        match self.next_player() {
            Player::A => (self.tiles_a, self.tiles_b),
            Player::B => (self.tiles_b, self.tiles_a),
        }
    }

    fn tiles_pov_mut(&mut self) -> (&mut Tiles, &mut Tiles) {
        match self.next_player {
            Player::A => (&mut self.tiles_a, &mut self.tiles_b),
            Player::B => (&mut self.tiles_b, &mut self.tiles_a),
        }
    }

    /// Set the correct outcome based on the current tiles and gaps.
    pub(super) fn update_outcome(&mut self) {
        let a_empty = self.tiles_a.is_empty();
        let b_empty = self.tiles_b.is_empty();

        let a_pass = self.must_pass(self.tiles_a);
        let b_pass = self.must_pass(self.tiles_b);

        let outcome = if self.moves_since_last_copy >= MAX_MOVES_SINCE_LAST_COPY || (a_empty && b_empty) {
            Some(Outcome::Draw)
        } else if a_empty {
            Some(Outcome::WonBy(Player::B))
        } else if b_empty {
            Some(Outcome::WonBy(Player::A))
        } else if a_pass && b_pass {
            let count_a = self.tiles_a.count();
            let count_b = self.tiles_b.count();

            let outcome = match count_a.cmp(&count_b) {
                Ordering::Less => Outcome::WonBy(Player::B),
                Ordering::Equal => Outcome::Draw,
                Ordering::Greater => Outcome::WonBy(Player::A),
            };
            Some(outcome)
        } else {
            None
        };

        self.outcome = outcome;
    }
}

impl Board for AtaxxBoard {
    type Move = Move;
    type Symmetry = D4Symmetry;

    fn can_lose_after_move() -> bool {
        true
    }

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        assert!(!self.is_done());

        let next_tiles = self.tiles_pov().0;

        match mv {
            Move::Pass => self.must_pass(next_tiles),
            Move::Copy { to } => (self.free_tiles() & next_tiles.copy_targets()).has(to),
            Move::Jump { from, to } => self.free_tiles().has(to) && next_tiles.has(from) && from.distance(to) == 2,
        }
    }

    fn random_available_move(&self, rng: &mut impl Rng) -> Self::Move {
        assert!(!self.is_done());

        let next_tiles = self.tiles_pov().0;
        let free_tiles = self.free_tiles();

        if self.must_pass(next_tiles) {
            return Move::Pass;
        }

        let copy_targets = self.free_tiles() & next_tiles.copy_targets();
        let jump_targets = free_tiles & next_tiles.jump_targets();

        let copy_count = copy_targets.count() as u32;
        let jump_count: u32 = jump_targets
            .into_iter()
            .map(|to| (next_tiles & Tiles::coord(to).jump_targets()).count() as u32)
            .sum();

        let index = rng.gen_range(0..(copy_count + jump_count));

        if index < copy_count {
            Move::Copy {
                to: copy_targets.get_nth(index),
            }
        } else {
            let mut left = index - copy_count;
            for to in jump_targets {
                let from = next_tiles & Tiles::coord(to).jump_targets();
                let count = from.count() as u32;
                if left < count {
                    let from = from.get_nth(left);
                    return Move::Jump { from, to };
                }
                left -= count;
            }

            unreachable!()
        }
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(self.is_available_move(mv), "{:?} is not available", mv);

        let (next_tiles, other_tiles) = self.tiles_pov_mut();

        let to = match mv {
            Move::Pass => {
                // we don't need to check whether the game is finished now because the other player is guaranteed to have
                //   a real move, since otherwise the game would have finished already
                self.next_player = self.next_player.other();
                return;
            }
            Move::Copy { to } => to,
            Move::Jump { from, to } => {
                *next_tiles &= !Tiles::coord(from);
                to
            }
        };

        let to = Tiles::coord(to);
        let converted = *other_tiles & to.copy_targets();
        *next_tiles |= to | converted;
        *other_tiles &= !converted;

        self.moves_since_last_copy += 1;
        if let Move::Copy { .. } = mv {
            self.moves_since_last_copy = 0;
        }

        self.update_outcome();
        self.next_player = self.next_player.other();
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn map(&self, sym: Self::Symmetry) -> Self {
        AtaxxBoard {
            tiles_a: self.tiles_a.map(sym),
            tiles_b: self.tiles_b.map(sym),
            gaps: self.gaps.map(sym),
            moves_since_last_copy: self.moves_since_last_copy,
            next_player: self.next_player,
            outcome: self.outcome,
        }
    }

    fn map_move(sym: Self::Symmetry, mv: Self::Move) -> Self::Move {
        match mv {
            Move::Pass => Move::Pass,
            Move::Copy { to } => Move::Copy { to: to.map(sym) },
            Move::Jump { from, to } => Move::Jump {
                from: from.map(sym),
                to: to.map(sym),
            },
        }
    }
}

#[derive(Debug)]
pub struct MoveIterator<'a> {
    board: &'a AtaxxBoard,
}

#[derive(Debug)]
pub struct AllMoveIterator;

impl<'a> BoardAvailableMoves<'a, AtaxxBoard> for AtaxxBoard {
    type MoveIterator = MoveIterator<'a>;
    type AllMoveIterator = AllMoveIterator;

    fn all_possible_moves() -> Self::AllMoveIterator {
        AllMoveIterator
    }

    fn available_moves(&'a self) -> Self::MoveIterator {
        assert!(!self.is_done());
        MoveIterator { board: self }
    }
}

impl<'a> InternalIterator for AllMoveIterator {
    type Item = Move;

    fn find_map<R, F>(self, mut f: F) -> Option<R>
    where
        F: FnMut(Self::Item) -> Option<R>,
    {
        if let Some(x) = f(Move::Pass) {
            return Some(x);
        };
        for to in Coord::all() {
            if let Some(x) = f(Move::Copy { to }) {
                return Some(x);
            };
        }
        for to in Coord::all() {
            for from in Tiles::coord(to).jump_targets() {
                if let Some(x) = f(Move::Jump { from, to }) {
                    return Some(x);
                };
            }
        }
        None
    }
}

impl<'a> InternalIterator for MoveIterator<'a> {
    type Item = Move;

    fn find_map<R, F>(self, mut f: F) -> Option<R>
    where
        F: FnMut(Self::Item) -> Option<R>,
    {
        let board = self.board;
        let next_tiles = board.tiles_pov().0;
        let free_tiles = board.free_tiles();

        // pass move, don't emit other moves afterwards
        if board.must_pass(next_tiles) {
            return f(Move::Pass);
        }

        // copy moves
        let copy_targets = free_tiles & next_tiles.copy_targets();
        for to in copy_targets {
            if let Some(x) = f(Move::Copy { to }) {
                return Some(x);
            }
        }

        // jump moves
        let jump_targets = free_tiles & next_tiles.jump_targets();
        for to in jump_targets {
            for from in next_tiles & Tiles::coord(to).jump_targets() {
                if let Some(x) = f(Move::Jump { from, to }) {
                    return Some(x);
                }
            }
        }

        None
    }
}
