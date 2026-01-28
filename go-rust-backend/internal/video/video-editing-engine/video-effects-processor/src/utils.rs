use std::sync::Once;

static INIT_LOGGER: Once = Once::new();

pub fn init_logging(level: i32) {
    INIT_LOGGER.call_once(|| {
        let env_level = match level {
            0 => return,
            1 => "error",
            2 => "warn",
            3 => "info",
            4 => "debug",
            _ => "trace",
        };

        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(env_level))
            .format_timestamp_millis()
            .init();
    });
}
