pub mod collector;
pub mod common;
pub mod config;
pub mod uplink;

mod utils;

use crate::{
    collector::{disk_free, exec, load_avg, memory, swap, Manager},
    config::Collectors,
};

pub fn create_from(config: Collectors) -> anyhow::Result<Manager> {
    let mut manager = Manager::new();

    if !config.memory.disabled {
        manager = manager.register("memory", memory::Collector);
    }
    if !config.swap.disabled {
        manager = manager.register("swap", swap::Collector);
    }
    if !config.disk_free.disabled {
        manager = manager.register("disk_free", disk_free::Collector);
    }
    if !config.load_avg.disabled {
        manager = manager.register("load_avg", load_avg::Collector);
    }
    if !config.exec.disabled {
        for (name, exec) in exec::Collector::new(config.exec) {
            manager = manager.register(name, exec);
        }
    }

    Ok(manager)
}
