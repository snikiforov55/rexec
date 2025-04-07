
use chrono::Utc;

pub fn time_stamp_fsec()->String{
    return Utc::now().format("%Y%m%d-%H%M%S%3f").to_string()
}

pub fn time_stamp_min()->String{
    return Utc::now().format("%Y%m%d-%H%M").to_string()
}