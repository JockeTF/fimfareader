//! Fimfareader GTK.

use fimfareader::prelude::*;

use components::AppWindow;

mod components;

fn main() -> Result<()> {
    gtk::init().map_err(|e| match e {
        _ => Error::usage("Could not initialize GTK"),
    })?;

    let _window = match AppWindow::new() {
        Some(window) => Ok(window),
        None => Err(Error::usage("Could not create main window")),
    }?;

    gtk::main();

    Ok(())
}
