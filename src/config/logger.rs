use log::error;
use log4rs;

pub struct Logger;

impl Logger {
    pub fn init() {
        if let Err(e) = log4rs::init_file("log4rs.yaml", Default::default()) {
            eprintln!("Failed to initialize logger : {}", e);
        };
    }
}

pub trait LogMsg {
    fn err(&self, msg: &str);
}

impl LogMsg for Logger {
    fn err(&self, msg: &str) {
        error!("{}", msg);
    }
}

#[cfg(test)]
mod tests_logger {
    use crate::config::logger::{LogMsg, Logger};

    #[test]
    fn test_log_error() {
        Logger::init();
        let logger = Logger;
        logger.err("Error message");
    }
}
