mod catalog;
mod config;
mod curves;
mod drafts;
mod environment;
mod extra;
mod localization;
mod paths;
mod ranges;
mod schema;
mod storage;
mod structured;
mod ui;
fn main() -> gtk::glib::ExitCode {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "-C" || arg == "--config-dir" {
            let Some(path) = args.next() else {
                eprintln!("--config-dir requires a directory");
                return gtk::glib::ExitCode::FAILURE;
            };
            std::env::set_var("LABWC_CONFIG_DIR", path);
        } else if arg == "--version" {
            println!("labwc-tweaks-gtk4 {}", env!("CARGO_PKG_VERSION"));
            return gtk::glib::ExitCode::SUCCESS;
        } else if arg == "--help" || arg == "-h" {
            println!(
                "Usage: labwc-tweaks-gtk4 [-C DIRECTORY | --config-dir DIRECTORY] [--version]"
            );
            return gtk::glib::ExitCode::SUCCESS;
        } else {
            eprintln!("Unknown option: {}", arg.to_string_lossy());
            return gtk::glib::ExitCode::FAILURE;
        }
    }
    use gtk::prelude::*;
    let app = gtk::Application::builder()
        .application_id("org.labwc.TweaksGtk4")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_startup(|_| gtk::Window::set_default_icon_name("org.labwc.TweaksGtk4"));
    app.connect_activate(|app| {
        if let Some(w) = app.active_window() {
            w.present();
            return;
        }
        ui::build(app);
    });
    app.run_with_args::<&str>(&[])
}
