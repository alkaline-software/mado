//! The core from a terminal, so a part runs and is timed without the
//! window:
//!
//!   mado-core paths                  where Mado keeps its files
//!   mado-core config                 the settings as the app reads them
//!   mado-core shortcuts              every action and its shortcut
//!   mado-core place <action> <window> <area> [<area>…]
//!                                    where an action puts a window, each
//!                                    frame as x,y,w,h in the Accessibility
//!                                    API's points (each area its screen's
//!                                    frame too)
//!   mado-core update                 the newest published version
//!   mado-core --try-update <version> <copy of Mado.app>

use mado_core::action::Action;
use mado_core::config::Config;
use mado_core::layout::{Memory, Rect, Screen};
use mado_core::{paths, update};

fn rect(text: &str) -> Option<Rect> {
    let v: Vec<f64> = text.split(',').map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
    (v.len() == 4).then(|| Rect::new(v[0], v[1], v[2], v[3]))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    match arg(0) {
        "paths" => {
            println!("home    {}", paths::root().display());
            println!("config  {}", paths::config_file().display());
            println!("state   {}", paths::state_dir().display());
            println!("cache   {}", paths::cache_dir().display());
        }
        "config" => println!("{}", serde_json::to_string_pretty(&Config::load()).unwrap_or_default()),
        "shortcuts" => {
            let cfg = Config::load();
            for a in Action::ALL {
                let s = cfg.shortcut(a).map(|s| s.to_string()).unwrap_or_else(|| "none".into());
                println!("{:<18} {s}", a.key());
            }
        }
        "place" => {
            let Some(action) = Action::from_key(arg(1)) else {
                eprintln!("mado-core: an action is one of {}", Action::ALL.map(|a| a.key()).join(", "));
                std::process::exit(2);
            };
            let frames: Vec<Rect> = args[2..].iter().filter_map(|a| rect(a)).collect();
            let (Some(window), screens) = (frames.first().copied(), frames.iter().skip(1).map(|a| Screen { frame: *a, area: *a }).collect::<Vec<_>>()) else {
                eprintln!("mado-core: place <action> x,y,w,h x,y,w,h [x,y,w,h…]");
                std::process::exit(2);
            };
            match Memory::default().plan(0, action, window, &screens) {
                Some(p) => println!("{},{},{},{}  on screen {}", p.frame.x, p.frame.y, p.frame.w, p.frame.h, p.screen),
                None => println!("nothing to do"),
            }
        }
        "update" => match update::latest() {
            Ok(v) => println!("{v} (this is {})", update::current_version()),
            Err(e) => {
                eprintln!("mado-core: {e}");
                std::process::exit(1);
            }
        },
        "--try-update" => {
            let (version, bundle) = (arg(1), arg(2));
            if version.is_empty() || bundle.is_empty() {
                eprintln!("mado-core: --try-update <version> <copy of Mado.app>");
                std::process::exit(2);
            }
            let done = update::download(version, &|done, total| eprint!("\r{done} of {}", total.map(|t| t.to_string()).unwrap_or("?".into()))).and_then(|file| update::install_into(&file, Some(std::path::Path::new(bundle))));
            eprintln!();
            if let Err(e) = done {
                eprintln!("mado-core: {e}");
                std::process::exit(1);
            }
            println!("{bundle} is now {version}");
        }
        _ => {
            eprintln!("mado-core paths | config | shortcuts | place <action> <window> <area>… | update | --try-update <version> <bundle>");
            std::process::exit(2);
        }
    }
}
