fn main() {
    if let Err(e) = review::app::init() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
