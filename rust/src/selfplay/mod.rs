use std::cmp::min;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Mutex;

use closure::closure;
use crossbeam::{channel, scope};
use crossbeam::channel::Sender;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rand::distributions::Distribution;
use rand::distributions::WeightedIndex;
use rand::Rng;
use sttt::board::{Board, Player};

mod collect;
pub mod generate_zero;
pub mod generate_mcts;

#[derive(Debug)]
pub struct Settings<G: Generator> {
    pub game_count: u64,
    pub output_path: String,

    pub move_selector: MoveSelector,
    pub generator: G,
}

pub trait Generator: Debug + Sync {
    type ThreadParam: Send;

    /// The parameter given to each launched thread.
    /// The length of the returned vec decides the number of threads launched;
    fn thread_params(&self) -> Vec<Self::ThreadParam>;

    fn thread_main(
        &self,
        move_selector: &MoveSelector,
        thread_param: Self::ThreadParam,
        start_counter: &StartGameCounter,
        sender: &Sender<Message>,
    );
}

#[derive(Debug)]
pub struct MoveSelector {
    //TODO this should be called zero_temp, inf temp is random play
    pub inf_temp_move_count: u32,

    //TODO add temperature?
    //TODO add dirichlet noise? or is that somewhere else?
}

#[derive(Debug)]
pub struct StartGameCounter {
    left: Mutex<u64>,
}

impl StartGameCounter {
    pub fn new(left: u64) -> Self {
        StartGameCounter { left: Mutex::new(left) }
    }

    #[must_use]
    pub fn request_up_to(&self, max_count: u64) -> u64 {
        let mut left = self.left.lock().unwrap();
        let picked = min(*left, max_count);
        *left -= picked;
        picked
    }
}


// A message sent back from a worker thread to the main collector thread.
#[derive(Debug)]
pub enum Message {
    Simulation(Simulation),
    Counter { evals: u64, moves: u64 },
}

// A full game.
#[derive(Debug, Clone)]
pub struct Simulation {
    won_by: Player,
    positions: Vec<Position>,
}

// A single position in a game.
#[derive(Debug, Clone)]
pub struct Position {
    board: Board,
    value: f32,
    policy: Vec<f32>,
}

impl<G: Generator> Settings<G> {
    pub fn run(&self) {
        println!("{:#?}", self);

        //open output file
        let output_path = PathBuf::from(&self.output_path);
        let output_folder = output_path.parent()
            .expect("Output should be in a folder");
        std::fs::create_dir_all(output_folder)
            .expect("Failed to create output directory");
        let file = File::create(&self.output_path)
            .expect("Failed to open output file");
        let mut writer = BufWriter::new(&file);

        let (sender, receiver) = channel::unbounded();
        let start_counter_left = StartGameCounter::new(self.game_count);

        scope(|s| {
            let thread_params = self.generator.thread_params();
            println!("Spawning {} threads", thread_params.len());

            for (i, thread_param) in thread_params.into_iter().enumerate() {
                s.builder()
                    .name(format!("worker-{}", i))
                    .spawn(closure!(
                            ref self.move_selector, ref start_counter_left, ref sender,
                            |_| {
                                // ignore "sender disconnected" errors, that just means
                                let _ = self.generator.thread_main(move_selector, thread_param, start_counter_left, sender);
                            }
                        ))
                    .expect("Failed to spawn thread");
            }

            println!("Start collecting");
            collect::collect(&mut writer, self.game_count, &receiver);

            //scope automatically joins the threads
            //  they should all stop automatically once there are no more games to start
        }).unwrap();
    }
}

impl MoveSelector {
    fn select(&self, move_count: u32, policy: &[f32], rng: &mut impl Rng) -> usize {
        if move_count > self.inf_temp_move_count {
            //pick the best move
            policy.iter().copied().map(OrderedFloat).position_max().unwrap()
        } else {
            //pick a random move following the policy
            let distr = WeightedIndex::new(policy).unwrap();
            distr.sample(rng)
        }
    }
}

