/*
 * Copyright (c) 2020. Stanislav Nikiforov
 */

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ProcessDescription {
    pub alias: String,
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_work_dir")]
    pub cwd: String,
    #[serde(default)]
    pub envs: HashMap<String, String>,
}

impl ProcessDescription {
    pub fn simple(alias: String,
                  program: String,
                  arguments: Vec<String>,
                  work_dir: String,
                  environment: HashMap<String, String>) -> Self {
        ProcessDescription {
            alias,
            cmd: program,
            args: arguments,
            cwd: work_dir,
            envs: environment
        }
    }

}

fn default_work_dir() -> String {
    ".".to_string()
}