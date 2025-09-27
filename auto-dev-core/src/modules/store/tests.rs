//! Tests for the local module store

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_store_config_default() {
        let config = StoreConfig::default();
        assert_eq!(config.store_path, PathBuf::from("./module_store"));
        assert_eq!(config.max_module_size, 100 * 1024 * 1024);
    }

    /// Create a test module directory with manifest
    fn create_test_module(dir: &TempDir, name: &str, version: &str) -> PathBuf {
        let module_dir = dir.path().join(format!("{}-{}", name, version));
        std::fs::create_dir_all(&module_dir).unwrap();

        let manifest = manifest::ModuleManifest {
            module: manifest::ModuleMetadata {
                name: name.to_string(),
                version: version.to_string(),
                description: format!("Test module {}", name),
                authors: vec!["test".to_string()],
                license: "MIT".to_string(),
                category: "utility".to_string(),
                repository: None,
                keywords: Some(vec![name.to_string(), "test".to_string()]),
                created: Some(chrono::Utc::now()),
                updated: Some(chrono::Utc::now()),
            },
            capabilities: manifest::ModuleCapabilities {
                provides: vec![format!("test:{}", name)],
                requires: vec!["base:capability".to_string()],
                optional: None,
            },
            compatibility: manifest::CompatibilityRequirements {
                auto_dev_version: ">=0.1.0".to_string(),
                platform: vec!["native".to_string()],
                rust_version: None,
                features: None,
            },
            verification: None,
            dependencies: None,
        };

        let manifest_path = module_dir.join("module.toml");
        manifest.save_to_file(&manifest_path).unwrap();

        // Create a dummy source file
        let src_dir = module_dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("lib.rs"), "// Test module").unwrap();

        module_dir
    }

    #[tokio::test]
    async fn test_module_store_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store");
        let install_path = temp_dir.path().join("modules");

        let config = StoreConfig {
            store_path: store_path.clone(),
            install_path: install_path.clone(),
            cache_path: temp_dir.path().join("cache"),
            verify_signatures: false,
            max_module_size: 100 * 1024 * 1024,
        };

        let mut store = ModuleStore::new(config).unwrap();

        // Create a test module
        let module_path = create_test_module(&temp_dir, "test-module", "1.0.0");

        // Add module to store
        let module_id = store.add_module(&module_path).await.unwrap();
        assert_eq!(module_id, "test-module-1_0_0");

        // Search for module
        let results = store.search("test").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].module.name, "test-module");

        // List all modules
        let all_modules = store.list_all().unwrap();
        assert_eq!(all_modules.len(), 1);

        // Get module by ID
        let module = store.get_module(&module_id).unwrap();
        assert_eq!(module.module.name, "test-module");
        assert_eq!(module.module.version, "1.0.0");

        // Install module
        let install_result = store.install(&module_id).await.unwrap();
        assert!(install_result.exists());
        assert!(install_result.join("module.toml").exists());

        // Remove module from store
        store.remove(&module_id).await.unwrap();

        // Verify module is removed
        let all_modules = store.list_all().unwrap();
        assert_eq!(all_modules.len(), 0);
    }

    #[tokio::test]
    async fn test_module_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store");

        let config = StoreConfig {
            store_path: store_path.clone(),
            install_path: temp_dir.path().join("modules"),
            cache_path: temp_dir.path().join("cache"),
            verify_signatures: false,
            max_module_size: 100 * 1024 * 1024,
        };

        let mut store = ModuleStore::new(config).unwrap();

        // Create multiple test modules
        let parser_module = create_test_module(&temp_dir, "python-parser", "1.0.0");
        let generator_module = create_test_module(&temp_dir, "code-generator", "2.0.0");
        let formatter_module = create_test_module(&temp_dir, "code-formatter", "1.5.0");

        // Update manifests with proper categories
        let parser_manifest_path = parser_module.join("module.toml");
        let mut parser_manifest =
            manifest::ManifestParser::parse_from_path(&parser_manifest_path).unwrap();
        parser_manifest.module.category = "parser".to_string();
        parser_manifest.save_to_file(&parser_manifest_path).unwrap();

        let gen_manifest_path = generator_module.join("module.toml");
        let mut gen_manifest =
            manifest::ManifestParser::parse_from_path(&gen_manifest_path).unwrap();
        gen_manifest.module.category = "generator".to_string();
        gen_manifest.save_to_file(&gen_manifest_path).unwrap();

        let fmt_manifest_path = formatter_module.join("module.toml");
        let mut fmt_manifest =
            manifest::ManifestParser::parse_from_path(&fmt_manifest_path).unwrap();
        fmt_manifest.module.category = "formatter".to_string();
        fmt_manifest.save_to_file(&fmt_manifest_path).unwrap();

        // Add modules to store
        store.add_module(&parser_module).await.unwrap();
        store.add_module(&generator_module).await.unwrap();
        store.add_module(&formatter_module).await.unwrap();

        // Test search functionality
        let python_results = store.search("python").unwrap();
        assert_eq!(python_results.len(), 1);
        assert_eq!(python_results[0].module.name, "python-parser");

        let code_results = store.search("code").unwrap();
        assert_eq!(code_results.len(), 2);

        // Test category filtering
        let parsers = store.list_by_category("parser").unwrap();
        assert_eq!(parsers.len(), 1);
        assert_eq!(parsers[0].module.name, "python-parser");

        let generators = store.list_by_category("generator").unwrap();
        assert_eq!(generators.len(), 1);
        assert_eq!(generators[0].module.name, "code-generator");
    }

    #[tokio::test]
    async fn test_module_installation() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store");
        let install_path = temp_dir.path().join("modules");

        let config = StoreConfig {
            store_path: store_path.clone(),
            install_path: install_path.clone(),
            cache_path: temp_dir.path().join("cache"),
            verify_signatures: false,
            max_module_size: 100 * 1024 * 1024,
        };

        let mut store = ModuleStore::new(config).unwrap();

        // Create and add a test module
        let module_path = create_test_module(&temp_dir, "installer-test", "1.0.0");
        let module_id = store.add_module(&module_path).await.unwrap();

        // Install the module
        let installed_path = store.install(&module_id).await.unwrap();

        // Verify installation
        assert!(installed_path.exists());
        assert!(installed_path.join("module.toml").exists());
        assert!(installed_path.join("src").join("lib.rs").exists());

        // Check that the installer tracked the installation
        let installer = installer::ModuleInstaller::new(install_path);
        assert!(installer.is_installed(&module_id));
    }

    #[test]
    fn test_module_manifest_validation() {
        // Test valid manifest
        let valid_toml = r#"
[module]
name = "test-module"
version = "1.0.0"
description = "Test module"
authors = ["test"]
license = "MIT"
category = "utility"

[capabilities]
provides = ["test:capability"]
requires = ["base:capability"]

[compatibility]
auto_dev_version = ">=0.1.0"
platform = ["native"]
        "#;

        let manifest = manifest::ManifestParser::parse(valid_toml).unwrap();
        assert_eq!(manifest.module.name, "test-module");

        // Test invalid version
        let invalid_version = r#"
[module]
name = "test-module"
version = "invalid"
description = "Test module"
authors = ["test"]
license = "MIT"
category = "utility"

[capabilities]
provides = ["test:capability"]
requires = ["base:capability"]

[compatibility]
auto_dev_version = ">=0.1.0"
platform = ["native"]
        "#;

        assert!(manifest::ManifestParser::parse(invalid_version).is_err());

        // Test invalid category
        let invalid_category = r#"
[module]
name = "test-module"
version = "1.0.0"
description = "Test module"
authors = ["test"]
license = "MIT"
category = "invalid-category"

[capabilities]
provides = ["test:capability"]
requires = ["base:capability"]

[compatibility]
auto_dev_version = ">=0.1.0"
platform = ["native"]
        "#;

        assert!(manifest::ManifestParser::parse(invalid_category).is_err());
    }

    #[tokio::test]
    async fn test_storage_manager() {
        let temp_dir = TempDir::new().unwrap();
        let store_root = temp_dir.path().to_path_buf();

        let mut storage = storage::StorageManager::new(store_root.clone()).unwrap();

        // Create a test module
        let module_dir = temp_dir.path().join("test-module");
        std::fs::create_dir_all(&module_dir).unwrap();

        let manifest = manifest::ModuleManifest::example();
        manifest.save_to_file(&module_dir.join("module.toml")).unwrap();
        std::fs::write(module_dir.join("test.rs"), "// Test").unwrap();

        // Store the module
        let module_id = storage.store_module(&module_dir, &manifest).await.unwrap();
        assert!(storage.exists(&module_id));

        // Get module info
        let info = storage.get_info(&module_id).unwrap();
        assert_eq!(info.name, "python-parser");
        assert_eq!(info.version, "1.0.0");

        // Get storage stats
        let stats = storage.get_stats();
        assert_eq!(stats.total_modules, 1);
        assert!(stats.total_size > 0);

        // Remove module
        storage.remove_module(&module_id).await.unwrap();
        assert!(!storage.exists(&module_id));
    }
}
