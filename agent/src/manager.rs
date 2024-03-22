use crate::collector::{self, Collector, Error};
use crate::command::{self, Command};
use crate::config::Commands;
use crate::{
    collector::{disk_free, load_avg, memory, swap},
    config::Collectors,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub struct Manager {
    pub collectors: HashMap<String, Box<dyn Collector>>,
    pub commands: HashMap<String, Box<dyn Command>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            collectors: Default::default(),
            commands: Default::default(),
        }
    }

    pub fn register_collector<N: Into<String>, C: Collector + 'static>(
        &mut self,
        name: N,
        collector: C,
    ) {
        self.collectors.insert(name.into(), Box::new(collector));
    }

    pub fn extend_collectors(
        &mut self,
        collectors: impl IntoIterator<Item = (impl Into<String>, impl Collector + 'static)>,
    ) {
        self.collectors.extend(
            collectors
                .into_iter()
                .map(|(k, v)| (k.into(), Box::new(v) as Box<dyn Collector + 'static>)),
        );
    }

    pub fn register_command<N: Into<String>, C: Command + 'static>(&mut self, name: N, command: C) {
        self.commands.insert(name.into(), Box::new(command));
    }

    pub fn extend_commands(
        &mut self,
        commands: impl IntoIterator<Item = (impl Into<String>, impl Command + 'static)>,
    ) {
        self.commands.extend(
            commands
                .into_iter()
                .map(|(k, v)| (k.into(), Box::new(v) as Box<dyn Command + 'static>)),
        );
    }

    pub async fn collect_one(&self, name: &str) -> Result<Option<serde_json::Value>, Error> {
        Ok(match self.collectors.get(name) {
            Some(collector) => Some(
                collector
                    .collect()
                    .await
                    .map_err(|err| Error::Collector(err.to_string()))?,
            ),
            None => None,
        })
    }

    pub async fn collect_all(&self) -> Result<BTreeMap<String, serde_json::Value>, Error> {
        let mut result = BTreeMap::new();
        for (name, collector) in &self.collectors {
            result.insert(
                name.to_string(),
                collector
                    .collect()
                    .await
                    .map_err(|err| Error::Collector(err.to_string()))?,
            );
        }
        Ok(result)
    }
}

impl TryFrom<(Collectors, Commands)> for Manager {
    type Error = anyhow::Error;

    fn try_from((collectors, commands): (Collectors, Commands)) -> anyhow::Result<Manager> {
        let mut manager = Manager::new();

        // collectors

        if !collectors.memory.disabled {
            manager.register_collector("memory", memory::Collector);
        }
        if !collectors.swap.disabled {
            manager.register_collector("swap", swap::Collector);
        }
        if !collectors.disk_free.disabled {
            manager.register_collector("disk_free", disk_free::Collector);
        }
        if !collectors.load_avg.disabled {
            manager.register_collector("load_avg", load_avg::Collector);
        }
        if !collectors.exec.disabled {
            manager.extend_collectors(collector::exec::Collector::new(collectors.exec));
        }

        // commands

        if !commands.exec.disabled {
            manager.extend_commands(command::exec::Command::new(commands.exec));
        }

        // return

        Ok(manager)
    }
}
