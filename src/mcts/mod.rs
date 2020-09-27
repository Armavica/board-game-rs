use ordered_float::OrderedFloat;
use rand::Rng;

use crate::board::{Board, Coord, Player};
use crate::bot_game::Bot;
use crate::mcts::heuristic::{Heuristic, ZeroHeuristic};

pub mod heuristic;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct IdxRange {
    start: usize,
    end: usize,
}

impl IdxRange {
    fn iter(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

impl IntoIterator for IdxRange {
    type Item = usize;
    type IntoIter = std::ops::Range<usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

struct Node {
    coord: Coord,
    parent: Option<usize>,
    children: Option<IdxRange>,
    visits: usize,
    wins: usize,
    needs_eval: bool,
}

impl Node {
    fn new(coord: Coord, parent: Option<usize>) -> Self {
        Node {
            coord,
            parent,
            children: None,
            visits: 0,
            wins: 0,
            needs_eval: false,
        }
    }

    fn uct(&self, parent_visits: usize, heuristic: f32) -> OrderedFloat<f32> {
        let wins = self.wins as f32;
        let visits = self.visits as f32;
        let value = (wins / visits) +
            (2.0 * (parent_visits as f32).ln() / visits).sqrt() +
            (heuristic / (visits + 1.0));
        value.into()
    }
}

#[derive(Debug)]
pub struct Evaluation {
    pub best_move: Option<Coord>,
    pub value: f32,
}

pub fn mcts_evaluate<H: Heuristic, R: Rng>(board: &Board, iterations: usize, heuristic: &H, batch_eval: bool, rand: &mut R) -> Evaluation {
    let mut tree: Vec<Node> = Vec::new();
    let mut eval_queue: Vec<(usize, Board)> = Vec::new();

    //the actual coord doesn't matter, just pick something
    tree.push(Node::new(Coord::from_o(0), None));

    for _ in 0..iterations {
        let mut curr_node = 0;
        let mut curr_board = board.clone();

        while !curr_board.is_done() {
            //Init children
            let children = match tree[curr_node].children {
                Some(children) => children,
                None => {
                    let start = tree.len();
                    tree.extend(curr_board.available_moves().map(|c| Node::new(c, Some(curr_node))));
                    let end = tree.len();

                    let children = IdxRange { start, end };
                    tree[curr_node].children = Some(children);
                    children
                }
            };

            //Exploration
            let unexplored_children = children.iter()
                .filter(|&c| tree[c].visits == 0);
            let count = unexplored_children.clone().count();

            if count != 0 {
                let child = unexplored_children.clone().nth(rand.gen_range(0, count))
                    .expect("we specifically selected the index based on the count already");

                curr_node = child;
                curr_board.play(tree[curr_node].coord);

                break;
            }

            //Selection
            let parent_visits = tree[curr_node].visits;

            let selected = children.iter().max_by_key(|&child| {
                let heuristic = heuristic.evaluate(&curr_board);
                tree[child].uct(parent_visits, heuristic)
            }).expect("Board is not done, this node should have a child");

            curr_node = selected;
            curr_board.play(tree[curr_node].coord);
        }

        //Simulate
        if batch_eval {
            if tree[curr_node].needs_eval {
                //do all queued evaluations
                // println!("Doing batch eval with queue size {}", eval_queue.len());
                do_batch_evals(&mut tree, &mut eval_queue, rand);
            } else {
                tree[curr_node].needs_eval = true;
                eval_queue.push((curr_node, curr_board));

                //only increment visit count here, win count will be incremented later in the batch
                loop {
                    tree[curr_node].visits += 1;

                    if let Some(parent) = tree[curr_node].parent {
                        curr_node = parent;
                    } else {
                        break;
                    }
                }
            }
        } else {
            let curr_player = curr_board.next_player;

            let won_by = loop {
                if let Some(won_by) = curr_board.won_by {
                    break won_by;
                }

                curr_board.play(curr_board.random_available_move(rand)
                    .expect("No winner, so board is not done yet"));
            };

            //Update
            let mut won = if won_by != Player::Neutral {
                won_by == curr_player
            } else {
                rand.gen()
            };

            loop {
                won = !won;

                let node = &mut tree[curr_node];
                node.visits += 1;
                if won {
                    node.wins += 1;
                }

                if let Some(parent) = node.parent {
                    curr_node = parent;
                } else {
                    break;
                }
            }
        }

        if batch_eval {
            if eval_queue.len() >= 100 {
                // println!("Trigger batch eval");
                do_batch_evals(&mut tree, &mut eval_queue, rand);
            }
        }
    }

    if batch_eval {
        // println!("Leftover queue: {}", eval_queue.len());
        do_batch_evals(&mut tree, &mut eval_queue, rand);
    }

    let best_move = match tree[0].children {
        None => board.random_available_move(rand),
        Some(children) => {
            children.iter().rev().max_by_key(|&child| {
                tree[child].visits
            }).map(|child| {
                tree[child].coord
            })
        }
    };

    let value = (tree[0].wins as f32) / (tree[0].visits as f32);
    Evaluation { best_move, value }
}

fn do_batch_evals(tree: &mut Vec<Node>, eval_queue: &mut Vec<(usize, Board)>, rand: &mut impl Rng) {
    for (mut curr_node, mut curr_board) in eval_queue.drain(..) {
        debug_assert!(tree[curr_node].needs_eval);
        let curr_player = curr_board.next_player;

        let won_by = loop {
            if let Some(won_by) = curr_board.won_by {
                break won_by;
            }

            curr_board.play(curr_board.random_available_move(rand)
                .expect("No winner, so board is not done yet"));
        };

        //Update
        let mut won = if won_by != Player::Neutral {
            won_by == curr_player
        } else {
            rand.gen()
        };

        tree[curr_node].needs_eval = false;

        loop {
            won = !won;

            let node = &mut tree[curr_node];
            if won {
                node.wins += 1;
            }

            if let Some(parent) = node.parent {
                curr_node = parent;
            } else {
                break;
            }
        }
    }
}

pub struct MCTSBot<H: Heuristic, R: Rng> {
    iterations: usize,
    heuristic: H,
    batch_eval: bool,
    rand: R,
}

impl<R: Rng> MCTSBot<ZeroHeuristic, R> {
    pub fn new(iterations: usize, rand: R) -> MCTSBot<ZeroHeuristic, R> {
        MCTSBot { iterations, heuristic: ZeroHeuristic, batch_eval: false, rand }
    }

    pub fn new_with_batch_eval(iterations: usize, rand: R) -> MCTSBot<ZeroHeuristic, R> {
        MCTSBot { iterations, heuristic: ZeroHeuristic, batch_eval: true, rand }
    }
}

impl<H: Heuristic, R: Rng> MCTSBot<H, R> {
    pub fn new_with_heuristic(iterations: usize, rand: R, heuristic: H) -> MCTSBot<H, R> {
        MCTSBot { iterations, heuristic, batch_eval: false, rand }
    }
}

impl<H: Heuristic, R: Rng> Bot for MCTSBot<H, R> {
    fn play(&mut self, board: &Board) -> Option<Coord> {
        mcts_evaluate(board, self.iterations, &self.heuristic, self.batch_eval, &mut self.rand).best_move
    }
}
