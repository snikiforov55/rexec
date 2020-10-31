
#[derive(Clone)]
pub struct Config{
    pub ip: String,
    pub port: u16,
    pub status_size: usize,
    pub stdout_size: usize,
}

impl Config{
    pub fn for_addr(ip: String, port: u16)->Self {
        Config{ip, port, status_size: 10, stdout_size: 10 }
    }
}