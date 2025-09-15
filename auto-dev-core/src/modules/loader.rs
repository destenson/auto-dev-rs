// Module Loader
//
// Handles loading and unloading of modules in various formats
// (WASM, native dynamic libraries, etc.)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::modules::interface::{ModuleInterface, ModuleMetadata};
use crate::modules::native_host::NativeModule;
use crate::modules::wasm_host::WasmModule;

/// Supported module formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ModuleFormat {
    Wasm,
    Native,
    Script, // Future: Python, JavaScript, etc.
}

impl ModuleFormat {
    pub fn from_extension(path: &Path) -> Option<Self> {
        path.extension().and_then(|ext| ext.to_str()).and_then(|ext| match ext {
            "wasm" => Some(ModuleFormat::Wasm),
            "so" | "dll" | "dylib" => Some(ModuleFormat::Native),
            _ => None,
        })
    }
}

/// Loaded module wrapper
pub enum LoadedModule {
    Wasm(WasmModule),
    Native(NativeModule),
}

impl LoadedModule {
    pub fn as_interface(&self) -> &dyn ModuleInterface {
        match self {
            LoadedModule::Wasm(m) => m,
            LoadedModule::Native(m) => m,
        }
    }

    pub fn as_interface_mut(&mut self) -> &mut dyn ModuleInterface {
        match self {
            LoadedModule::Wasm(m) => m,
            LoadedModule::Native(m) => m,
        }
    }

    pub fn metadata(&self) -> ModuleMetadata {
        self.as_interface().metadata()
    }
}

/// Module loader cache entry
struct CacheEntry {
    module: LoadedModule,
    path: PathBuf,
    format: ModuleFormat,
    loaded_at: chrono::DateTime<chrono::Utc>,
}

/// Module loader that manages loading/unloading of modules
pub struct ModuleLoader {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    wasm_engine: Arc<wasmtime::Engine>,
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new() -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);

        let engine = wasmtime::Engine::new(&config)?;

        Ok(Self { cache: Arc::new(RwLock::new(HashMap::new())), wasm_engine: Arc::new(engine) })
    }

    /// Load a module from the specified path
    pub async fn load(&self, path: PathBuf, format: ModuleFormat) -> Result<LoadedModule> {
        // Check if module is already loaded
        let cache_key = path.to_string_lossy().to_string();

        {
            let cache = self.cache.read().await;
            if cache.contains_key(&cache_key) {
                anyhow::bail!("Module already loaded from path: {}", path.display());
            }
        }

        // Load based on format
        let module = match format {
            ModuleFormat::Wasm => {
                let wasm_module = WasmModule::load(&path, &self.wasm_engine)
                    .await
                    .context("Failed to load WASM module")?;
                LoadedModule::Wasm(wasm_module)
            }
            ModuleFormat::Native => {
                let native_module =
                    NativeModule::load(&path).context("Failed to load native module")?;
                LoadedModule::Native(native_module)
            }
            ModuleFormat::Script => {
                anyhow::bail!("Script modules not yet implemented");
            }
        };

        // Cache the loaded module
        let entry =
            CacheEntry { module, path: path.clone(), format, loaded_at: chrono::Utc::now() };

        let mut cache = self.cache.write().await;
        let module_id = entry.module.metadata().name.clone();
        cache.insert(module_id.clone(), entry);

        // Return the module
        Ok(cache.get(&module_id).unwrap().module.clone())
    }

    /// Unload a module
    pub async fn unload(&self, module_id: &str) -> Result<()> {
        let mut cache = self.cache.write().await;

        if let Some(mut entry) = cache.remove(module_id) {
            // Shutdown the module gracefully
            entry.module.as_interface_mut().shutdown().await?;
            Ok(())
        } else {
            anyhow::bail!("Module not found: {}", module_id)
        }
    }

    /// Get the format of a loaded module
    pub fn get_format(&self, module_id: &str) -> Result<ModuleFormat> {
        let cache = futures::executor::block_on(self.cache.read());

        cache
            .get(module_id)
            .map(|entry| entry.format)
            .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))
    }

    /// List all loaded modules
    pub async fn list_loaded(&self) -> Vec<String> {
        self.cache.read().await.keys().cloned().collect()
    }

    /// Get a loaded module by ID
    pub async fn get_module(&self, module_id: &str) -> Option<LoadedModule> {
        self.cache.read().await.get(module_id).map(|entry| entry.module.clone())
    }

    /// Validate a module before loading
    pub async fn validate(&self, path: &Path, format: ModuleFormat) -> Result<()> {
        if !path.exists() {
            anyhow::bail!("Module file does not exist: {}", path.display());
        }

        match format {
            ModuleFormat::Wasm => {
                // Validate WASM module
                let bytes = tokio::fs::read(path).await?;
                wasmtime::Module::validate(&self.wasm_engine, &bytes)
                    .context("Invalid WASM module")?;
            }
            ModuleFormat::Native => {
                // Basic validation for native modules
                if !path.is_file() {
                    anyhow::bail!("Native module path is not a file");
                }
            }
            ModuleFormat::Script => {
                anyhow::bail!("Script validation not yet implemented");
            }
        }

        Ok(())
    }

    /// Reload a module with a new version
    pub async fn reload(&self, module_id: &str, new_path: PathBuf) -> Result<()> {
        let format = self.get_format(module_id)?;

        // Validate new module
        self.validate(&new_path, format).await?;

        // Get current module state
        let state = {
            let cache = self.cache.read().await;
            let entry = cache
                .get(module_id)
                .ok_or_else(|| anyhow::anyhow!("Module not found: {}", module_id))?;
            entry.module.as_interface().get_state()?
        };

        // Unload old module
        self.unload(module_id).await?;

        // Load new module
        let mut new_module = self.load(new_path, format).await?;

        // Restore state
        new_module.as_interface_mut().restore_state(state)?;

        Ok(())
    }
}

// Clone implementation for LoadedModule (needed for cache)
impl Clone for LoadedModule {
    fn clone(&self) -> Self {
        match self {
            LoadedModule::Wasm(m) => LoadedModule::Wasm(m.clone()),
            LoadedModule::Native(m) => LoadedModule::Native(m.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.wasm")),
            Some(ModuleFormat::Wasm)
        );
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.dll")),
            Some(ModuleFormat::Native)
        );
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.so")),
            Some(ModuleFormat::Native)
        );
        assert_eq!(ModuleFormat::from_extension(Path::new("module.txt")), None);
    }

    #[tokio::test]
    async fn test_loader_creation() {
        let loader = ModuleLoader::new();
        assert!(loader.is_ok());
    }
}
