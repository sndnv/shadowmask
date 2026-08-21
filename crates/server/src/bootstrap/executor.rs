use std::future::Future;
use std::path::Path;

use super::env::load_expanded;
use super::{BootstrapError, BootstrapResult, Created};

pub trait BootstrapEntityProvider {
    type Entity;

    fn name(&self) -> &'static str;

    fn defaults(&self) -> Vec<Self::Entity> {
        Vec::new()
    }

    fn load(&self, value: &toml::Value) -> Result<Self::Entity, BootstrapError>;

    fn validate(&self, entities: &[Self::Entity]) -> Result<(), BootstrapError>;

    fn create(
        &self,
        entity: Self::Entity,
    ) -> impl Future<Output = Result<Created, BootstrapError>> + Send;

    fn render(&self, entity: &Self::Entity) -> String;

    fn extract_id(&self, entity: &Self::Entity) -> String;
}

pub async fn run_one<P>(provider: &P, dir: &Path) -> BootstrapResult
where
    P: BootstrapEntityProvider + Sync,
    P::Entity: Send,
{
    let name = provider.name();
    let path = dir.join(format!("{name}.toml"));

    let root = match load_expanded(&path) {
        Ok(root) => root,
        Err(error) => {
            tracing::error!(provider = name, %error, "bootstrap: failed to load config file");
            return BootstrapResult::empty();
        }
    };

    let section = root
        .get(name)
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut entities = provider.defaults();
    for value in &section {
        match provider.load(value) {
            Ok(entity) => entities.push(entity),
            Err(error) => {
                tracing::error!(provider = name, %error, "bootstrap: failed to parse entry");
                return BootstrapResult::empty();
            }
        }
    }

    if let Err(error) = provider.validate(&entities) {
        tracing::error!(provider = name, %error, "bootstrap: validation failed");
        return BootstrapResult::empty();
    }

    let found = entities.len();
    let mut created = 0;
    let mut skipped = 0;
    for entity in entities {
        let id = provider.extract_id(&entity);
        tracing::debug!(provider = name, entity = %provider.render(&entity), "bootstrap: creating entity");
        match provider.create(entity).await {
            Ok(Created::New) => {
                created += 1;
                tracing::info!(provider = name, id, "bootstrap: created entity");
            }
            Ok(Created::Skipped) => {
                skipped += 1;
                tracing::info!(
                    provider = name,
                    id,
                    "bootstrap: entity already exists, skipped"
                );
            }
            Err(error) => {
                tracing::error!(provider = name, id, %error, "bootstrap: failed to create entity");
            }
        }
    }

    tracing::info!(
        provider = name,
        found,
        created,
        skipped,
        "bootstrap: provider finished"
    );

    BootstrapResult {
        found,
        created,
        skipped,
    }
}

pub fn complete(total: BootstrapResult) -> BootstrapResult {
    tracing::info!(
        found = total.found,
        created = total.created,
        skipped = total.skipped,
        "bootstrap: complete"
    );
    total
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::super::require_unique;
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Widget {
        id: String,
        label: String,
    }

    struct WidgetProvider {
        store: Mutex<Vec<Widget>>,
        preexisting: Vec<String>,
        defaults: Vec<Widget>,
    }

    impl WidgetProvider {
        fn new() -> Self {
            Self {
                store: Mutex::new(Vec::new()),
                preexisting: Vec::new(),
                defaults: Vec::new(),
            }
        }

        fn created(&self) -> Vec<Widget> {
            self.store.lock().unwrap().clone()
        }
    }

    impl BootstrapEntityProvider for WidgetProvider {
        type Entity = Widget;

        fn name(&self) -> &'static str {
            "widgets"
        }

        fn defaults(&self) -> Vec<Widget> {
            self.defaults.clone()
        }

        fn load(&self, value: &toml::Value) -> Result<Widget, BootstrapError> {
            let field = |key: &str| {
                value
                    .get(key)
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| BootstrapError::Invalid {
                        entity: "widget",
                        reason: format!("missing string field {key}"),
                    })
            };
            Ok(Widget {
                id: field("id")?,
                label: field("label")?,
            })
        }

        fn validate(&self, entities: &[Widget]) -> Result<(), BootstrapError> {
            require_unique(entities, "widget", "id", |w| w.id.clone())
        }

        async fn create(&self, entity: Widget) -> Result<Created, BootstrapError> {
            if self.preexisting.contains(&entity.id) {
                return Ok(Created::Skipped);
            }
            self.store.lock().unwrap().push(entity);
            Ok(Created::New)
        }

        fn render(&self, entity: &Widget) -> String {
            format!("{}={}", entity.id, entity.label)
        }

        fn extract_id(&self, entity: &Widget) -> String {
            entity.id.clone()
        }
    }

    fn write_widgets(dir: &Path, body: &str) {
        std::fs::write(dir.join("widgets.toml"), body).unwrap();
    }

    #[tokio::test]
    async fn creates_configured_entities() {
        let dir = tempfile::tempdir().unwrap();
        write_widgets(
            dir.path(),
            "[[widgets]]\nid = \"a\"\nlabel = \"Alpha\"\n\n[[widgets]]\nid = \"b\"\nlabel = \"Beta\"\n",
        );
        let provider = WidgetProvider::new();
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.found, 2);
        assert_eq!(result.created, 2);
        assert_eq!(result.skipped, 0);
        assert_eq!(provider.created().len(), 2);
    }

    #[tokio::test]
    async fn a_config_file_that_cannot_be_read_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("widgets.toml")).unwrap();
        let mut provider = WidgetProvider::new();
        provider.defaults = vec![Widget {
            id: "built-in".to_owned(),
            label: "Built In".to_owned(),
        }];

        let result = run_one(&provider, dir.path()).await;

        assert_eq!(result.found, 0);
        assert_eq!(result.created, 0);
        assert!(
            provider.created().is_empty(),
            "an unreadable file must not fall back to the defaults, which would hide it"
        );
    }

    #[tokio::test]
    async fn missing_file_creates_only_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let mut provider = WidgetProvider::new();
        provider.defaults = vec![Widget {
            id: "default".to_owned(),
            label: "Default".to_owned(),
        }];
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.found, 1);
        assert_eq!(result.created, 1);
        assert_eq!(provider.created(), provider.defaults);
    }

    #[tokio::test]
    async fn skips_preexisting_entities() {
        let dir = tempfile::tempdir().unwrap();
        write_widgets(dir.path(), "[[widgets]]\nid = \"a\"\nlabel = \"Alpha\"\n");
        let mut provider = WidgetProvider::new();
        provider.preexisting = vec!["a".to_owned()];
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result.found, 1);
        assert_eq!(result.created, 0);
        assert_eq!(result.skipped, 1);
        assert!(provider.created().is_empty());
    }

    #[tokio::test]
    async fn duplicate_ids_fail_validation() {
        let dir = tempfile::tempdir().unwrap();
        write_widgets(
            dir.path(),
            "[[widgets]]\nid = \"a\"\nlabel = \"one\"\n\n[[widgets]]\nid = \"a\"\nlabel = \"two\"\n",
        );
        let provider = WidgetProvider::new();
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result, BootstrapResult::empty());
        assert!(provider.created().is_empty());
    }

    #[tokio::test]
    async fn parse_failure_aborts_provider() {
        let dir = tempfile::tempdir().unwrap();
        write_widgets(dir.path(), "[[widgets]]\nlabel = \"no id\"\n");
        let provider = WidgetProvider::new();
        let result = run_one(&provider, dir.path()).await;
        assert_eq!(result, BootstrapResult::empty());
    }

    #[tokio::test]
    async fn results_from_several_providers_add_up() {
        let dir = tempfile::tempdir().unwrap();
        write_widgets(dir.path(), "[[widgets]]\nid = \"a\"\nlabel = \"Alpha\"\n");

        let first = run_one(&WidgetProvider::new(), dir.path()).await;
        let second = run_one(&WidgetProvider::new(), dir.path()).await;
        let total = complete(first + second);

        assert_eq!(total.found, 2);
        assert_eq!(total.created, 2);
    }
}
