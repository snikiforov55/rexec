
use chrono::Utc;

pub fn time_stamp_fsec()->String{
    return Utc::now().format("%Y%m%d-%H%M%S%3f").to_string()
}

pub fn time_stamp_sec()->String{
    return Utc::now().format("%Y%m%d-%H%M%S").to_string()
}
pub fn time_stamp_min()->String{
    return Utc::now().format("%Y%m%d-%H%M").to_string()
}
pub fn time_stamp_hour()->String{
    return Utc::now().format("%Y%m%d-%H").to_string()
}