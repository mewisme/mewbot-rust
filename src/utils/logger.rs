#![allow(dead_code)]

use chrono::Local;

mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const UNDERLINE: &str = "\x1b[4m";

    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    pub const BRIGHT_BLACK: &str = "\x1b[90m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";
    pub const BRIGHT_WHITE: &str = "\x1b[97m";
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Error,
    Warn,
    Debug,
    Done,
}

impl LogLevel {
    fn color(&self) -> &'static str {
        use colors::*;
        match self {
            LogLevel::Info => BRIGHT_BLUE,
            LogLevel::Error => BRIGHT_RED,
            LogLevel::Warn => BRIGHT_YELLOW,
            LogLevel::Debug => BRIGHT_MAGENTA,
            LogLevel::Done => BRIGHT_GREEN,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Error => "ERRO",
            LogLevel::Warn => "WARN",
            LogLevel::Debug => "DBUG",
            LogLevel::Done => "DONE",
        }
    }
}

fn format_timestamp() -> String {
    let now = Local::now();
    now.format("%H:%M:%S").to_string()
}

#[macro_export]
macro_rules! log_location {
    () => {
        (file!(), line!())
    };
}

pub fn log_internal(level: LogLevel, message: &str, file: &str, line: u32) {
    use colors::*;

    let timestamp = format_timestamp();
    let color = level.color();
    let label = level.label();

    let filename = file
        .split('/')
        .last()
        .or_else(|| file.split('\\').last())
        .unwrap_or(file);

    println!(
        "{}{}{}{} {}{}{}:{}{} {}{}[{}]{} {}",
        UNDERLINE,
        BRIGHT_BLACK,
        timestamp,
        RESET,
        UNDERLINE,
        BRIGHT_BLACK,
        filename,
        line,
        RESET,
        BOLD,
        color,
        label,
        RESET,
        message
    );
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_internal(
            $crate::utils::logger::LogLevel::Info,
            &format!($($arg)*),
            file!(),
            line!()
        )
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_internal(
            $crate::utils::logger::LogLevel::Error,
            &format!($($arg)*),
            file!(),
            line!()
        )
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_internal(
            $crate::utils::logger::LogLevel::Warn,
            &format!($($arg)*),
            file!(),
            line!()
        )
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_internal(
            $crate::utils::logger::LogLevel::Debug,
            &format!($($arg)*),
            file!(),
            line!()
        )
    };
}

#[macro_export]
macro_rules! done {
    ($($arg:tt)*) => {
        $crate::utils::logger::log_internal(
            $crate::utils::logger::LogLevel::Done,
            &format!($($arg)*),
            file!(),
            line!()
        )
    };
}
