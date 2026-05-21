//! Route registration for plugin-provided pages

/// A route registration for a plugin-provided page
///
/// v1 uses string identifiers — the UI maps IDs to actual route components
#[derive(Clone)]
pub struct RouteRegistration {
    /// URL prefix for this plugin's routes (e.g., "/my-plugin")
    pub prefix: String,
    /// Route identifier — UI maps this to actual component
    pub route_id: String,
}

/// Registry for plugin-provided routes
#[derive(Default)]
pub struct RouteRegistry {
    routes: Vec<RouteRegistration>,
}

impl RouteRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a route
    pub fn register(&mut self, route: RouteRegistration) {
        // Validate prefix starts with /
        if !route.prefix.starts_with('/') {
            panic!(
                "Route prefix must start with '/', got: {}",
                route.prefix
            );
        }

        self.routes.push(route);
    }

    /// Get all registered routes in registration order
    pub fn routes(&self) -> &[RouteRegistration] {
        &self.routes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_route() {
        let mut registry = RouteRegistry::new();
        registry.register(RouteRegistration {
            prefix: "/my-plugin".into(),
            route_id: "my-plugin-page".into(),
        });

        assert_eq!(registry.routes().len(), 1);
        assert_eq!(registry.routes()[0].prefix, "/my-plugin");
    }

    #[test]
    fn test_multiple_routes() {
        let mut registry = RouteRegistry::new();
        registry.register(RouteRegistration {
            prefix: "/plugin-a".into(),
            route_id: "plugin-a-page".into(),
        });
        registry.register(RouteRegistration {
            prefix: "/plugin-b".into(),
            route_id: "plugin-b-page".into(),
        });

        assert_eq!(registry.routes().len(), 2);
    }
}
