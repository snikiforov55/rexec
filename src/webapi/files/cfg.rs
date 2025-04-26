use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub(super) struct SaveOptions {
    pub create_dir: Option<bool>,
    pub replace_file: Option<bool>,
}
impl SaveOptions {
    pub fn default() -> SaveOptions {
        Self {
            create_dir: Some(true),
            replace_file: Some(false),
        }
    }
}