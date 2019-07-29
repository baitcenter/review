fn main() {
    env_logger::init();
    if let Err(e) = review::app::init() {
        review::log_error(&e);
        std::process::exit(1);
    }
}
