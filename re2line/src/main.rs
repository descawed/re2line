use std::env;
use std::path::PathBuf;

mod app;
mod collision;
mod compare;
mod rdt;
mod script;
mod aot;
mod character;
mod record;
mod draw;
mod rng;

fn make_eframe_error(e: anyhow::Error) -> eframe::Error {
    eframe::Error::AppCreation(std::io::Error::new(std::io::ErrorKind::Other, e).into())
}

fn main() -> eframe::Result {
    //rng::sim::simulate_bus_rng();
    //rng::sim::find_runs();
    //rng::sim::simulate_pre_bus_rng();
    //rng::sim::print_gate_shots();
    //rng::sim::simulate_bus_manip(500, 566);
    //return Ok(());
    
    let args: Vec<String> = env::args().collect();

    let mut app = app::App::new().map_err(make_eframe_error)?;
    if args.len() > 1 {
        app.load_game_folder(PathBuf::from(&args[1])).map_err(make_eframe_error)?;
    } else {
        // if we bail on this error then it'll be impossible to start the app without manually
        // editing the config file
        if let Err(e) = app.try_resume() {
            eprintln!("Failed to load previous game folder: {}", e);
        }
    }
    
    eframe::run_native(app::APP_NAME, eframe::NativeOptions::default(), Box::new(|_| Ok(Box::new(app))))
}
