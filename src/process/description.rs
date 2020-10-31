use std::collections::HashMap;

#[derive(Clone)]
pub struct ProcessDescription {
    pub alias: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub work_dir: String,
    pub environment: HashMap<String, String>,
}

impl ProcessDescription {
    pub fn simple(alias: String,
                  program: String,
                  arguments: Vec<String>,
                  work_dir: String,
                  environment: HashMap<String, String>) -> Self {
        ProcessDescription { alias, command: program, arguments, work_dir, environment}
    }

}