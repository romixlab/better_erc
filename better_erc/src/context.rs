use erc_core::PcbAssembly;
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

    pub fn blocking_read(&self) -> RwLockReadGuard<'_, ContextShared> {
        self.shared.blocking_read()
    }

    pub fn blocking_write(&mut self) -> RwLockWriteGuard<'_, ContextShared> {
        self.shared.blocking_write()
    }
}

#[derive(Default)]
pub struct ContextShared {
    pub boards: Vec<PcbAssembly>,
}
