use ansi_term::Color::*;
use chrono::{Datelike, Local, Timelike};
use failure::Fail;
use log::{error, info};
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::{
    fmt::Display,
    fs::{self, File, OpenOptions},
    io::{Error as IoError, ErrorKind, Write},
    process,
    sync::Mutex,
};

pub trait UnwrapOrLog<T> {
    fn unwrap_or_log(self, message: impl Display) -> T;
}

impl<T, E> UnwrapOrLog<T> for Result<T, E>
where
    E: Display,
{
    fn unwrap_or_log(self, message: impl Display) -> T {
        match self {
            Ok(val) => val,
            Err(err) => {
                error!("{}: {}", message, err);
                process::exit(1)
            }
        }
    }
}

impl<T> UnwrapOrLog<T> for Option<T> {
    fn unwrap_or_log(self, message: impl Display) -> T {
        match self {
            Some(val) => val,
            None => {
                error!("{}", message);
                process::exit(1)
            }
        }
    }
}

#[derive(Debug, Fail)]
pub enum LoggerInitError {
    #[fail(display = "Failed create logs folder: {}", err)]
    LogsFolder { err: IoError },
    #[fail(display = "Failed create latest log: {}", err)]
    LatestLog { err: IoError },
    #[fail(display = "Failed create archived log: {}", err)]
    ArchivedLog { err: IoError },
}

pub struct Logger {
    latest_log: Mutex<File>,
    archived_log: Mutex<File>,
    console_colored: bool,
}

impl Logger {
    pub fn init(console_colored: bool) -> Result<(), LoggerInitError> {
        if let Err(err) = fs::create_dir("logs") {
            match err.kind() {
                ErrorKind::AlreadyExists => (),
                _ => return Err(LoggerInitError::LogsFolder { err }),
            }
        }

        let latest_log = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("latest_log.txt")
        {
            Ok(file) => file,
            Err(err) => return Err(LoggerInitError::LatestLog { err }),
        };

        let time_log_path = format!("./logs/{}-log.txt", format_yyyymmdd_hhmmss());
        let time_log = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&time_log_path)
        {
            Ok(file) => file,
            Err(err) => return Err(LoggerInitError::ArchivedLog { err }),
        };;

        let logger = Logger {
            latest_log: Mutex::new(latest_log),
            archived_log: Mutex::new(time_log),
            console_colored,
        };

        if log::set_boxed_logger(Box::new(logger)).is_ok() {
            log::set_max_level(LevelFilter::Info)
        }

        info!("Logger initialized");

        Ok(())
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let time = format_hhmmssnnn();
            let uncolored_output = output(&time, record, false);

            // ignore errors
            (
                writeln!(self.latest_log.lock().unwrap(), "{}", uncolored_output),
                writeln!(self.archived_log.lock().unwrap(), "{}", uncolored_output),
            );

            let console_ouput = if self.console_colored {
                output(&time, record, true)
            } else {
                uncolored_output
            };

            println!("{}", console_ouput);
        }
    }

    // we cant deal with any errors, so we ignore them
    fn flush(&self) {
        if let Ok(mut file) = self.latest_log.lock() {
            let _ = file.flush();
        }

        if let Ok(mut file) = self.archived_log.lock() {
            let _ = file.flush();
        }
    }
}

fn output(time: &str, record: &log::Record, colored: bool) -> String {
    if colored {
        let level = match record.level() {
            Level::Error => Red.paint("ERROR"),
            Level::Warn => Yellow.paint("WARN"),
            Level::Info => Green.paint("INFO"),
            Level::Debug => Purple.paint("DEBUG"),
            Level::Trace => Cyan.paint("TRACE"),
        };

        // https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg
        format!(
            "[{}] {}:{} - {}: {}",
            Fixed(245).paint(time),
            record.module_path().unwrap_or_default(),
            record
                .line()
                .map(|option| Fixed(172).paint(option.to_string()).to_string())
                .unwrap_or_default(),
            level,
            record.args(),
        )
    } else {
        format!(
            "[{}] {}:{} - {}: {}",
            time,
            record.module_path().unwrap_or_default(),
            record.line().unwrap_or_default(),
            record.level(),
            record.args(),
        )
    }
}

fn format_hhmmssnnn() -> String {
    let time = Local::now();

    format!(
        "{:02}:{:02}:{:02}.{:03}",
        time.hour(),
        time.minute(),
        time.second(),
        time.nanosecond() / 1_000_000,
    )
}

fn format_yyyymmdd_hhmmss() -> String {
    let time = Local::now();

    format!(
        "{:04}-{:02}-{:02}-{:02}.{:02}.{:02}",
        time.year(),
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    )
}
