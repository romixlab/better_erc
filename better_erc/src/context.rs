use ecad_file_format::pcb_assembly::PcbAssembly;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone)]
pub struct Context {
    shared: Arc<RwLock<ContextShared>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(RwLock::new(Default::default())),
        }
    }

    pub fn blocking_read(&self) -> RwLockReadGuard<ContextShared> {
        self.shared.blocking_read()
    }

    pub fn blocking_write(&mut self) -> RwLockWriteGuard<ContextShared> {
        self.shared.blocking_write()
    }
}

#[derive(Default)]
pub struct ContextShared {
    pub boards: Vec<PcbAssembly>,
}
