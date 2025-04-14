// /*
//  * Copyright (c) 2020. Stanislav Nikiforov
//  */
use std::{collections::HashMap, sync::Arc};

use log::debug;
use tokio::sync::RwLock;

use crate::proc::{comm::Process, description::ProcessDescription};

pub struct Register {
    procs: HashMap<String, Process>,
}
impl Register {
    pub fn new() -> Register {
        Register {
            procs: HashMap::new(),
        }
    }
    pub fn add(&mut self, proc: Process) {
        debug!("Add process {proc}");
        self.procs.insert(proc.desc.alias.clone(), proc);
    }
    pub fn get(&self, alias: &String) -> Option<&Process> {
        self.procs.get(alias)
    }
    pub fn get_mut(&mut self, alias: &String) -> Option<&mut Process> {
        self.procs.get_mut(alias)
    }
    pub fn get_all_desc(&self) -> Vec<ProcessDescription> {
        self.procs.values().map(|p| p.desc.clone()).collect()
    }
    pub fn remove(&mut self, alias: &String) {
        self.procs.remove(alias);
    }
}
pub type RegisterRef = Arc<RwLock<Register>>;
pub fn create_register_ref() -> RegisterRef {
    Arc::new(RwLock::new(Register::new()))
}
