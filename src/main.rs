use std::fs::File;
use std::io::BufReader;

use rfd::FileDialog;

mod collision;
mod rdt;
mod view;
mod math;
mod script;

fn main() -> eframe::Result {
    let file = FileDialog::new()
        .add_filter("RDTs", &["rdt", "RDT"])
        .set_directory("/media/jacob/E2A6DD85A6DD5A9D/games/BIOHAZARD 2 PC/pl0/Rdt") // TODO: remove after testing
        .pick_file()
        ;

    let Some(file) = file else {
        return Ok(());
    };

    let rdt = match File::open(file).map_err(anyhow::Error::new).and_then(|f| {
        let reader = BufReader::new(f);
        rdt::Rdt::read(reader)
    }) {
        Ok(rdt) => rdt,
        Err(e) => return Err(eframe::Error::AppCreation(std::io::Error::new(std::io::ErrorKind::Other, e).into())),
    };

    let view = view::View::new(rdt);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1920.0, 1080.0]),
        ..Default::default()
    };

    eframe::run_native("re2line", options, Box::new(|_| Ok(Box::new(view))))
}
