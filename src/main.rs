use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use rfd::FileDialog;

mod app;
mod collision;
mod rdt;
mod math;
mod script;
mod aot;

fn make_eframe_error(e: anyhow::Error) -> eframe::Error {
    eframe::Error::AppCreation(std::io::Error::new(std::io::ErrorKind::Other, e).into())
}

fn main() -> eframe::Result {
    let args: Vec<String> = env::args().collect();

    let file = if args.len() == 2 {
        PathBuf::from(args[1].clone())
    } else {
        match FileDialog::new()
            .add_filter("RDTs", &["rdt", "RDT"])
            .set_directory("/media/jacob/E2A6DD85A6DD5A9D/games/BIOHAZARD 2 PC/pl0/Rdt") // TODO: remove after testing
            .pick_file() {
            Some(path) => path,
            None => return Ok(()),
        }
    };

    let rdt = match File::open(file).map_err(anyhow::Error::new).and_then(|f| {
        let reader = BufReader::new(f);
        rdt::Rdt::read(reader)
    }) {
        Ok(rdt) => rdt,
        Err(e) => return Err(make_eframe_error(e)),
    };

    let app = app::App::new(rdt).map_err(make_eframe_error)?;
    eframe::run_native(app::APP_NAME, eframe::NativeOptions::default(), Box::new(|_| Ok(Box::new(app))))
}
