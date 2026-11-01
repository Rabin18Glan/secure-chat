use dotenv;

pub fn init() {
    dotenv::dotenv().ok();
    dotenv::from_filename("server/.env").ok();
    dotenv::from_filename(".env").ok();
}

#[allow(dead_code)]
pub fn get(parameter: &str) -> String {
    std::env::var(parameter)
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_else(|_| panic!("{} is not defined in the environment.", parameter))
}

pub fn get_or(parameter: &str, default: &str) -> String {
    std::env::var(parameter)
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_else(|_| default.to_string())
}
