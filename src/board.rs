#![allow(dead_code)]

use std::fmt::{self, Write, Debug};

use rand::Rng;

use crate::board::Player::Neutral;
use itertools::Itertools;

#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(Eq, PartialEq)]
pub enum Player {
    Player,
    Enemy,
    Neutral,
}

impl Player {
    pub fn other(self) -> Player {
        match self {
            Player::Player => Player::Enemy,
            Player::Enemy => Player::Player,
            Player::Neutral => Player::Neutral,
        }
    }

    fn index(self) -> u32 {
        match self {
            Player::Player => 0,
            Player::Enemy => 1,
            Player::Neutral => panic!(),
        }
    }
}

#[derive(Copy, Clone)]
#[derive(Eq, PartialEq)]
pub struct Coord(u8);

impl Coord {
//    pub fn none() -> Coord {
//        Coord(100)
//    }

    pub fn of_oo(om: u8, os: u8) -> Coord {
       debug_assert!(om < 9);
       debug_assert!(os < 9);
        Coord(9 * om + os)
    }

    pub fn of_o(o: u8) -> Coord {
       debug_assert!(o < 81);
        Coord::of_oo(o / 9, o % 9)
    }

    pub fn of_xy(x: u8, y: u8) -> Coord {
       debug_assert!(x < 9 && y < 9);
        Coord(((x / 3) + (y / 3) * 3) * 9 + ((x % 3) + (y % 3) * 3))
    }

    pub fn om(self) -> u8 {
        self.0 / 9
    }

    pub fn os(self) -> u8 {
        self.0 % 9
    }

    pub fn o(self) -> u8 {
        9 * self.om() + self.os()
    }

    pub fn x(self) -> u8 {
        (self.om() % 3) * 3 + (self.os() % 3)
    }

    pub fn y(self) -> u8 {
        (self.om() / 3) * 3 + (self.os() / 3)
    }
}

impl Debug for Coord {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "({}, {})", self.om(), self.os())
    }
}

//TODO implement simpler partialeq again
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Board {
    grids: [u32; 9],
    main_grid: u32,

    pub last_move: Option<Coord>,
    pub next_player: Player,
    pub won_by: Option<Player>,

    macro_mask: u32,
    macro_open: u32,

//    hash: usize,
}

// impl PartialEq for Board {
//     fn eq(&self, other: &Self) -> bool {
//         self.grids == other.grids && self.macro_mask == other.macro_mask
//     }
// }

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..9 {
            if y == 3 || y == 6 {
                f.write_str("---+---+---\n")?;
            }

            for x in 0..9 {
                if x == 3 || x == 6 {
                    f.write_char('|')?;
                }

                let coord = Coord::of_xy(x, y);
                let symbol = match (Some(coord) == self.last_move, self.tile(coord)) {
                    (false, Player::Player) => 'X',
                    (true, Player::Player) => 'x',
                    (false, Player::Enemy) => 'O',
                    (true, Player::Enemy) => 'o',
                    (_, Player::Neutral) => ' ',
                };

                f.write_char(symbol)?;
            }

            f.write_char('\n')?;
        }

        Ok(())
    }
}

pub struct BoardMoveIterator<'a> {
    board: &'a Board,
    macro_left: u32,
    curr_om: u32,
    grid_left: u32,
}

impl<'a> BoardMoveIterator<'a> {
    fn empty(board: &Board) -> BoardMoveIterator {
        BoardMoveIterator { board, macro_left: 0, curr_om: 0, grid_left: 0 }
    }
    fn new(board: &Board) -> BoardMoveIterator {
        BoardMoveIterator { board, macro_left: board.macro_mask, curr_om: 0, grid_left: 0 }
    }
}

impl<'a> Iterator for BoardMoveIterator<'a> {
    type Item = Coord;

    fn next(&mut self) -> Option<Coord> {
        if self.grid_left == 0 {
            if self.macro_left == 0 {
                return None;
            } else {
                self.curr_om = self.macro_left.trailing_zeros();
                self.macro_left &= self.macro_left - 1;
                self.grid_left = !compact_grid(self.board.grids[self.curr_om as usize]) & Board::FULL_MASK;
            }
        }

        let os = self.grid_left.trailing_zeros();
        self.grid_left &= self.grid_left - 1;

        Some(Coord::of_oo(self.curr_om as u8, os as u8))
    }
}

lazy_static! {
    static ref PIECE_BITS: [usize; 2*81 + 9] = {
        let mut arr = [0; 2*81 + 9];
        let mut rng = rand::thread_rng();
        for i in 0..arr.len() {
            arr[i] = rng.gen();
        }
        arr
    };
}

fn get_piece_bit(coord: Coord, player: Player) -> usize {
    match player {
        Player::Player => PIECE_BITS[coord.o() as usize],
        Player::Enemy => PIECE_BITS[81 + coord.o() as usize],
        Player::Neutral => panic!(),
    }
}

fn get_last_move_bit(coord: Coord) -> usize {
    PIECE_BITS[2 * 81 + coord.os() as usize]
}

//impl Hash for Board {
//    fn hash<H: Hasher>(&self, state: &mut H) {
//        state.write_usize(self.hash)
//    }
//}

impl Board {
    const FULL_MASK: u32 = 0b111_111_111;

    pub fn new() -> Board {
        Board {
            grids: [0; 9],
            main_grid: 0,
            last_move: None,
            next_player: Player::Player,
            won_by: None,
            macro_mask: Board::FULL_MASK,
            macro_open: Board::FULL_MASK,
//            hash: 0,
        }
    }

    pub fn is_done(&self) -> bool {
        self.won_by != None
    }

    pub fn tile(&self, coord: Coord) -> Player {
        get_player(self.grids[coord.om() as usize], coord.os())
    }

    pub fn macr(&self, om: u8) -> Player {
        get_player(self.main_grid, om)
    }

    pub fn available_moves(&self) -> impl Iterator<Item=Coord> + '_ {
        return if self.is_done() {
            BoardMoveIterator::empty(&self)
        } else {
            BoardMoveIterator::new(&self)
        };
    }

//    pub fn get_hash(&self) -> usize {
//        return self.hash;
//    }

    // #[inline(always)]
    pub fn random_available_move<R: Rng>(&self, rand: &mut R) -> Option<Coord> {
        if self.is_done() {
            return None;
        }

        let mut count = 0;
        for om in BitIter::of(self.macro_mask) {
            count += 9 - self.grids[om as usize].count_ones();
        }

        let mut index = rand.gen_range(0, count);

        for om in BitIter::of(self.macro_mask) {
            let grid = self.grids[om as usize];
            let grid_count = 9 - grid.count_ones();

            if index < grid_count {
                let os = get_nth_set_bit(!compact_grid(grid), index as u32);
                return Some(Coord::of_oo(om as u8, os as u8));
            }

            index -= grid_count;
        }

        //todo try unchecked here
        unreachable!()
    }

    pub fn is_available_move(&self, coord: Coord) -> bool {
        let om = coord.om();
        let os = coord.os();
        has_bit(self.macro_mask, om) &&
            !has_bit(compact_grid(self.grids[om as usize]), os)
    }

    pub fn play(&mut self, coord: Coord) -> bool {
       debug_assert!(!self.is_done(), "can't play on done board");
       debug_assert!(self.is_available_move(coord), "move not available");

        //do actual move
        let won_grid = self.set_tile_and_update(self.next_player, coord);

        //update for next player
        self.last_move = Some(coord);
        self.next_player = self.next_player.other();

        won_grid
    }

    fn set_tile_and_update(&mut self, player: Player, coord: Coord) -> bool {
        let om = coord.om();
        let os = coord.os();
        let p = (9 * player.index()) as u8;

        //update hash
//        self.hash ^= get_piece_bit(coord, player);
//        self.hash ^= self.last_move.map_or(0, |m| get_last_move_bit(m));
//        self.hash ^= get_last_move_bit(coord);

        //set tile and macro, check win
        let new_grid = self.grids[om as usize] | (1 << (os + p));
        self.grids[om as usize] = new_grid;

        let grid_win = is_win_grid(new_grid >> p);
        if grid_win {
            let new_main_grid = self.main_grid | (1 << (om + p));
            self.main_grid = new_main_grid;

            if is_win_grid(new_main_grid >> p) {
                self.won_by = Some(player);
            }
        }

        //update macro masks, remove bit from open and recalculate mask
        if grid_win || new_grid.count_ones() == 9 {
            self.macro_open &= !(1 << om);
            if self.macro_open == 0 && self.won_by.is_none() {
                self.won_by = Some(Neutral);
            }
        }
        self.macro_mask = self.calc_macro_mask(os);

        grid_win
    }

    fn calc_macro_mask(&self, os: u8) -> u32 {
        if has_bit(self.macro_open, os) {
            1u32 << os
        } else {
            self.macro_open
        }
    }
}

fn is_win_grid(grid: u32) -> bool {
    let grid = grid & Board::FULL_MASK;

    const WIN_GRIDS: [u32; 16] = [2155905152, 4286611584, 4210076288, 4293962368, 3435954304, 4291592320, 4277971584, 4294748800, 2863300736, 4294635760, 4210731648, 4294638320, 4008607872, 4294897904, 4294967295, 4294967295];
    has_bit(WIN_GRIDS[(grid / 32) as usize], (grid % 32) as u8)
}

fn has_bit(x: u32, i: u8) -> bool {
    ((x >> i) & 1) != 0
}

fn has_mask(x: u32, mask: u32) -> bool {
    x & mask == mask
}

fn get_nth_set_bit(mut x: u32, n: u32) -> u32 {
    for _ in 0..n {
        x &= x.wrapping_sub(1);
    }
    x.trailing_zeros()
}

fn compact_grid(grid: u32) -> u32 {
    (grid | grid >> 9) & Board::FULL_MASK
}

fn get_player(grid: u32, index: u8) -> Player {
    if has_bit(grid, index) {
        Player::Player
    } else if has_bit(grid, index + 9) {
        Player::Enemy
    } else {
        Player::Neutral
    }
}

struct BitIter {
    left: u32
}

impl BitIter {
    fn of(int: u32) -> BitIter {
        BitIter { left: int }
    }
}

impl Iterator for BitIter {
    type Item = u32;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.left == 0 {
            None
        } else {
            let index = self.left.trailing_zeros();
            self.left &= self.left - 1;
            Some(index)
        }
    }
}

pub fn board_to_compact_string(board: &Board) -> String {
    (0..81).map(|o| {
        let coord = Coord::of_o(o);
        match (Some(coord) == board.last_move, board.tile(coord)) {
            (false, Player::Player) => 'X',
            (true, Player::Player) => 'x',
            (false, Player::Enemy) => 'O',
            (true, Player::Enemy) => 'o',
            (_, Player::Neutral) => ' ',
        }
    }).join("")
}

pub fn board_from_compact_string(s: &str) -> Board {
    debug_assert!(s.chars().count() == 81);

    let mut board = Board::new();
    let mut last_move = None;

    for (o, c) in s.chars().enumerate() {
        let coord = Coord::of_o(o as u8);

        let (player, last) = match c {
            'X' => (Player::Player, false),
            'x' => (Player::Player, true),
            'O' => (Player::Enemy, false),
            'o' => (Player::Enemy, true),
            ' ' => (Player::Neutral, false),
            _ => panic!("unexpected character in shortstring")
        };

        if last {
            last_move = Some((player, coord));
        }

        if player != Player::Neutral {
            board.set_tile_and_update(player, coord);
        }
    }

    if let Some((last_player, last_coord)) = last_move {
        board.set_tile_and_update(last_player, last_coord);
        board.last_move = Some(last_coord);
        board.next_player = last_player.other()
    }

    board
}