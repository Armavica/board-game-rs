use sttt_zero::selfplay::Settings;

fn main() {
    let settings = Settings {
        devices: Settings::all_cuda_devices(),
        threads_per_device: 2,
        batch_size: 400,

        position_count: 100_000,
        output_path: "../data/loop/games.bin".to_owned(),

        network_path: "../data/esat/trained_model_10_epochs.pt".to_owned(),
        iterations: 1000,
        exploration_weight: 1.0,
        inf_temp_move_count: 20,
    };
    settings.run();
}
