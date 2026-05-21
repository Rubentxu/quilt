//! Slash commands and registry for command palette extension

/// A slash command available in the command palette
pub struct SlashCommand {
    /// Unique identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Emoji or short text icon
    pub icon: String,
    /// Short description shown in palette
    pub description: String,
    /// Action to execute when command is selected
    /// The string input is the text after the slash command trigger
    /// Returns true if handled, false to bubble
    pub action: Box<dyn Fn(&str) -> bool + Send + Sync>,
}

impl SlashCommand {
    /// Create a new slash command
    pub fn new(
        id: &str,
        label: &str,
        icon: &str,
        description: &str,
        action: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            description: description.into(),
            action: Box::new(action),
        }
    }

    /// Execute this command with the given input
    pub fn execute(&self, input: &str) -> bool {
        (self.action)(input)
    }
}

/// Registry for slash commands
#[derive(Default)]
pub struct SlashCommandRegistry {
    commands: Vec<SlashCommand>,
}

impl SlashCommandRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a slash command
    ///
    /// If a command with the same `id` already exists, the new command is skipped.
    pub fn register_slash_command(&mut self, cmd: SlashCommand) {
        // Skip duplicates
        if self.commands.iter().any(|c| c.id == cmd.id) {
            tracing::warn!(
                "Skipping duplicate SlashCommand with id: {}, label: {}",
                cmd.id,
                cmd.label
            );
            return;
        }

        self.commands.push(cmd);
    }

    /// Get all registered commands in registration order
    pub fn commands(&self) -> &[SlashCommand] {
        &self.commands
    }

    /// Find a command by id
    pub fn find(&self, id: &str) -> Option<&SlashCommand> {
        self.commands.iter().find(|c| c.id == id)
    }

    /// Filter commands by query string (matches label or description)
    pub fn filter(&self, query: &str) -> Vec<&SlashCommand> {
        let q = query.to_lowercase();
        self.commands
            .iter()
            .filter(|cmd| {
                cmd.label.to_lowercase().contains(&q)
                    || cmd.description.to_lowercase().contains(&q)
            })
            .collect()
    }
}

impl Clone for SlashCommand {
    fn clone(&self) -> Self {
        // Note: action is not clonable, so we wrap in a no-op for Clone
        // This is only used in testing scenarios where action isn't invoked
        Self {
            id: self.id.clone(),
            label: self.label.clone(),
            icon: self.icon.clone(),
            description: self.description.clone(),
            action: Box::new(|_| true), // no-op action for cloned commands
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_action(_: &str) -> bool {
        true
    }

    #[test]
    fn test_register_command() {
        let mut registry = SlashCommandRegistry::new();
        registry.register_slash_command(SlashCommand::new(
            "test",
            "Test Command",
            "⚡",
            "A test",
            dummy_action,
        ));

        assert_eq!(registry.commands().len(), 1);
        assert_eq!(registry.commands()[0].id, "test");
    }

    #[test]
    fn test_find() {
        let mut registry = SlashCommandRegistry::new();
        registry.register_slash_command(SlashCommand::new(
            "search",
            "Search",
            "🔍",
            "Search the graph",
            dummy_action,
        ));

        let found = registry.find("search");
        assert!(found.is_some());
        assert_eq!(found.unwrap().label, "Search");
    }

    #[test]
    fn test_filter_matches_label() {
        let mut registry = SlashCommandRegistry::new();
        registry.register_slash_command(SlashCommand::new(
            "search",
            "Search",
            "🔍",
            "Search the graph",
            dummy_action,
        ));
        registry.register_slash_command(SlashCommand::new(
            "graph",
            "Graph View",
            "🌐",
            "Open graph",
            dummy_action,
        ));

        let results = registry.filter("sea");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "search");
    }

    #[test]
    fn test_filter_matches_description() {
        let mut registry = SlashCommandRegistry::new();
        registry.register_slash_command(SlashCommand::new(
            "search",
            "Search",
            "🔍",
            "Search the graph",
            dummy_action,
        ));

        let results = registry.filter("graph");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "search");
    }

    #[test]
    fn test_execute() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static CALLED: AtomicBool = AtomicBool::new(false);

        let cmd = SlashCommand::new("test", "Test", "⚡", "Test", |_| {
            CALLED.store(true, Ordering::SeqCst);
            true
        });

        let result = cmd.execute("input");
        assert!(CALLED.load(Ordering::SeqCst));
        assert!(result);
    }
}
