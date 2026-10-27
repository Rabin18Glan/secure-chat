use dotenv;

pub fn init() {
    dotenv::dotenv().ok();
}

#[allow(dead_code)]
pub fn get(parameter: &str) -> String {
    std::env::var(parameter)
        .unwrap_or_else(|_| panic!("{} is not defined in the environment.", parameter))
}

pub fn get_or(parameter: &str, default: &str) -> String {
    std::env::var(parameter).unwrap_or_else(|_| default.to_string())
}
