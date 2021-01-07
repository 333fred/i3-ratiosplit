use ini::{Ini, Properties};
use log::LevelFilter;

const DEFAULT_RATIO: f64 = 0.33;
const DEFAULT_LOG_PATH: &str = "~/.config/i3/ratiosplit.log";
const DEFAULT_LOG_FILE_LEVEL: LevelFilter = LevelFilter::Info;
const DEFAULT_LOG_CONSOLE_LEVEL: LevelFilter = LevelFilter::Off;

#[derive(Debug)]
pub struct Settings {
    pub ratio: f64,
    pub log_file_level: LevelFilter,
    pub log_file: String,
    pub log_console_level: LevelFilter,
}

pub fn load_settings() -> Settings {
    let conf_file = match Ini::load_from_file(
        shellexpand::full("~/.config/i3/ratiosplit.ini")
            .unwrap()
            .to_string(),
    ) {
        Ok(file) => file,
        Err(err) => {
            println!("Error {:?} loading settings, using defaults", err);
            return default_settings();
        }
    };

    let main_section = match conf_file.section(Some("main")) {
        Some(s) => s,
        None => {
            println!("No main section found in config, using defaults");
            return default_settings();
        }
    };

    let ratio = match main_section.get("ratio") {
        Some(ratio_string) => ratio_string.parse::<f64>().unwrap_or(DEFAULT_RATIO),
        None => DEFAULT_RATIO,
    };

    let log_file = main_section
        .get("log_file")
        .unwrap_or(DEFAULT_LOG_PATH)
        .to_string();

    let log_file_level = get_level(main_section, "log_file_level", DEFAULT_LOG_FILE_LEVEL);
    let log_console_level = get_level(main_section, "log_console_level", DEFAULT_LOG_CONSOLE_LEVEL);

    return Settings {
        ratio,
        log_file: shellexpand::full(log_file.as_str()).unwrap().to_string(),
        log_file_level,
        log_console_level,
    };

    fn get_level(main_section: &Properties, path: &str, default: LevelFilter) -> LevelFilter {
        match main_section.get(path) {
            None => default,
            Some(level_str) => match level_str.parse() {
                Ok(l) => l,
                Err(_) => default,
            },
        }
    }
}

fn default_settings() -> Settings {
    Settings {
        ratio: DEFAULT_RATIO,
        log_file: shellexpand::full(DEFAULT_LOG_PATH).unwrap().to_string(),
        log_file_level: DEFAULT_LOG_FILE_LEVEL,
        log_console_level: DEFAULT_LOG_CONSOLE_LEVEL,
    }
}
