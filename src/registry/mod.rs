use crate::commands::Command;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Registry {
    commands: HashMap<String, Arc<dyn Command>>,
    prefix_map: HashMap<String, Arc<dyn Command>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            prefix_map: HashMap::new(),
        }
    }

    pub fn register(&mut self, command: Arc<dyn Command>) {
        let name = command.name().to_lowercase();
        let command_clone = command.clone();
        self.commands.insert(name.clone(), command_clone);

        if let Some(prefix) = command.prefix() {
            let prefix_lower = prefix.to_lowercase();
            self.prefix_map.insert(prefix_lower, command.clone());

            if !self.prefix_map.contains_key(&name) {
                self.prefix_map.insert(name.clone(), command.clone());
            }
        } else {
            if !self.prefix_map.contains_key(&name) {
                self.prefix_map.insert(name.clone(), command.clone());
            }
        }

        for alias in command.aliases() {
            let alias_lower = alias.to_lowercase();
            if !self.prefix_map.contains_key(&alias_lower) {
                self.prefix_map.insert(alias_lower, command.clone());
            }
        }
    }

    pub fn get_slash_command(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.commands.get(&name.to_lowercase()).cloned()
    }

    pub fn get_prefix_command(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.prefix_map.get(&name.to_lowercase()).cloned()
    }

    pub fn all_commands(&self) -> Vec<Arc<dyn Command>> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for (name, cmd) in &self.commands {
            if seen.insert(name.clone()) {
                result.push(cmd.clone());
            }
        }

        result
    }

    #[allow(dead_code)]
    pub fn refresh(&mut self) {
        self.commands.clear();
        self.prefix_map.clear();
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
