// Native Module Host
//
// Provides execution environment for native dynamic library modules

use anyhow::{Context, Result};
use async_trait::async_trait;
use libloading::{Library, Symbol};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

use crate::modules::interface::{
    ModuleCapability, ModuleInterface, ModuleMetadata, ModuleState, ModuleVersion,
};

/// Type aliases for module functions
type InitializeFn = unsafe extern "C" fn() -> i32;
type ExecuteFn = unsafe extern "C" fn(*const u8, usize, *mut u8, *mut usize) -> i32;
type GetMetadataFn = unsafe extern "C" fn(*mut u8, *mut usize) -> i32;
type GetCapabilitiesFn = unsafe extern "C" fn(*mut u8, *mut usize) -> i32;
type ShutdownFn = unsafe extern "C" fn() -> i32;
type GetStateFn = unsafe extern "C" fn(*mut u8, *mut usize) -> i32;
type RestoreStateFn = unsafe extern "C" fn(*const u8, usize) -> i32;

/// Native module wrapper
#[derive(Clone)]
pub struct NativeModule {
    library: Arc<Library>,
    metadata: ModuleMetadata,
    state: ModuleState,
}

impl NativeModule {
    /// Load a native module from a dynamic library
    pub fn load(path: &Path) -> Result<Self> {
        // Load the library
        let library = unsafe { Library::new(path).context("Failed to load native library")? };

        // Get metadata from the module
        let metadata = Self::get_module_metadata(&library)?;
        let state = ModuleState::new(metadata.version.clone());

        Ok(Self { library: Arc::new(library), metadata, state })
    }

    /// Get metadata from the native module
    fn get_module_metadata(library: &Library) -> Result<ModuleMetadata> {
        unsafe {
            let get_metadata: Symbol<GetMetadataFn> = library
                .get(b"module_get_metadata")
                .context("Module doesn't export 'module_get_metadata'")?;

            let mut buffer = vec![0u8; 4096];
            let mut size = buffer.len();

            let result = get_metadata(buffer.as_mut_ptr(), &mut size);
            if result != 0 {
                anyhow::bail!("Failed to get module metadata: error code {}", result);
            }

            buffer.truncate(size);
            let metadata: ModuleMetadata =
                serde_json::from_slice(&buffer).context("Failed to deserialize module metadata")?;

            Ok(metadata)
        }
    }

    /// Call a function in the native module
    fn call_function(&self, func_name: &str, input: &[u8]) -> Result<Vec<u8>> {
        unsafe {
            let func: Symbol<ExecuteFn> = self
                .library
                .get(func_name.as_bytes())
                .with_context(|| format!("Function '{}' not found in module", func_name))?;

            let mut output_buffer = vec![0u8; 65536]; // 64KB buffer
            let mut output_size = output_buffer.len();

            let result =
                func(input.as_ptr(), input.len(), output_buffer.as_mut_ptr(), &mut output_size);

            if result != 0 {
                anyhow::bail!("Function '{}' failed with error code: {}", func_name, result);
            }

            output_buffer.truncate(output_size);
            Ok(output_buffer)
        }
    }
}

#[async_trait]
impl ModuleInterface for NativeModule {
    fn metadata(&self) -> ModuleMetadata {
        self.metadata.clone()
    }

    async fn initialize(&mut self) -> Result<()> {
        unsafe {
            if let Ok(init_fn) = self.library.get::<InitializeFn>(b"module_initialize") {
                let result = init_fn();
                if result != 0 {
                    anyhow::bail!("Module initialization failed with code: {}", result);
                }
            }
        }
        Ok(())
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let input_bytes = serde_json::to_vec(&input)?;
        let output_bytes = self.call_function("module_execute", &input_bytes)?;

        if output_bytes.is_empty() {
            Ok(Value::Null)
        } else {
            let output: Value = serde_json::from_slice(&output_bytes)?;
            Ok(output)
        }
    }

    fn get_capabilities(&self) -> Vec<ModuleCapability> {
        unsafe {
            if let Ok(get_caps) = self.library.get::<GetCapabilitiesFn>(b"module_get_capabilities")
            {
                let mut buffer = vec![0u8; 4096];
                let mut size = buffer.len();

                if get_caps(buffer.as_mut_ptr(), &mut size) == 0 {
                    buffer.truncate(size);
                    if let Ok(caps) = serde_json::from_slice(&buffer) {
                        return caps;
                    }
                }
            }
        }
        self.metadata.capabilities.clone()
    }

    async fn handle_message(&mut self, message: Value) -> Result<Option<Value>> {
        let input_bytes = serde_json::to_vec(&message)?;

        match self.call_function("module_handle_message", &input_bytes) {
            Ok(output_bytes) => {
                if output_bytes.is_empty() {
                    Ok(None)
                } else {
                    let output: Value = serde_json::from_slice(&output_bytes)?;
                    Ok(Some(output))
                }
            }
            Err(e) if e.to_string().contains("not found") => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn shutdown(&mut self) -> Result<()> {
        unsafe {
            if let Ok(shutdown_fn) = self.library.get::<ShutdownFn>(b"module_shutdown") {
                let result = shutdown_fn();
                if result != 0 {
                    anyhow::bail!("Module shutdown failed with code: {}", result);
                }
            }
        }
        Ok(())
    }

    fn get_state(&self) -> Result<ModuleState> {
        unsafe {
            if let Ok(get_state) = self.library.get::<GetStateFn>(b"module_get_state") {
                let mut buffer = vec![0u8; 8192];
                let mut size = buffer.len();

                if get_state(buffer.as_mut_ptr(), &mut size) == 0 {
                    buffer.truncate(size);
                    let state: ModuleState = serde_json::from_slice(&buffer)?;
                    return Ok(state);
                }
            }
        }
        Ok(self.state.clone())
    }

    fn restore_state(&mut self, state: ModuleState) -> Result<()> {
        unsafe {
            if let Ok(restore_state) = self.library.get::<RestoreStateFn>(b"module_restore_state") {
                let state_bytes = serde_json::to_vec(&state)?;
                let result = restore_state(state_bytes.as_ptr(), state_bytes.len());

                if result != 0 {
                    anyhow::bail!("Failed to restore module state: error code {}", result);
                }
            }
        }
        self.state = state;
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        match self.call_function("module_health_check", &[]) {
            Ok(result) => {
                if result.is_empty() {
                    Ok(true)
                } else {
                    let healthy: bool = serde_json::from_slice(&result).unwrap_or(true);
                    Ok(healthy)
                }
            }
            Err(e) if e.to_string().contains("not found") => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Helper module for creating native modules in Rust
/// This can be used as a template for module developers
#[cfg(feature = "module-sdk")]
pub mod sdk {
    use super::*;
    use std::ffi::c_void;

    /// Macro to generate module exports
    #[macro_export]
    macro_rules! export_module {
        ($module_type:ty) => {
            static mut MODULE_INSTANCE: Option<$module_type> = None;

            #[no_mangle]
            pub extern "C" fn module_initialize() -> i32 {
                unsafe {
                    MODULE_INSTANCE = Some(<$module_type>::new());
                    0
                }
            }

            #[no_mangle]
            pub extern "C" fn module_get_metadata(buffer: *mut u8, size: *mut usize) -> i32 {
                let metadata = <$module_type>::metadata();
                let json = match serde_json::to_vec(&metadata) {
                    Ok(v) => v,
                    Err(_) => return 1,
                };

                unsafe {
                    if json.len() > *size {
                        return 2;
                    }
                    std::ptr::copy_nonoverlapping(json.as_ptr(), buffer, json.len());
                    *size = json.len();
                }
                0
            }

            #[no_mangle]
            pub extern "C" fn module_execute(
                input: *const u8,
                input_len: usize,
                output: *mut u8,
                output_size: *mut usize,
            ) -> i32 {
                unsafe {
                    if let Some(ref module) = MODULE_INSTANCE {
                        let input_slice = std::slice::from_raw_parts(input, input_len);
                        let input_value: Value = match serde_json::from_slice(input_slice) {
                            Ok(v) => v,
                            Err(_) => return 1,
                        };

                        let result = match futures::executor::block_on(module.execute(input_value))
                        {
                            Ok(v) => v,
                            Err(_) => return 2,
                        };

                        let output_json = match serde_json::to_vec(&result) {
                            Ok(v) => v,
                            Err(_) => return 3,
                        };

                        if output_json.len() > *output_size {
                            return 4;
                        }

                        std::ptr::copy_nonoverlapping(
                            output_json.as_ptr(),
                            output,
                            output_json.len(),
                        );
                        *output_size = output_json.len();
                        0
                    } else {
                        5
                    }
                }
            }

            #[no_mangle]
            pub extern "C" fn module_shutdown() -> i32 {
                unsafe {
                    MODULE_INSTANCE = None;
                    0
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_module_metadata() {
        let metadata = ModuleMetadata {
            name: "native_test".to_string(),
            version: ModuleVersion::new(1, 0, 0),
            author: "test".to_string(),
            description: "Native test module".to_string(),
            capabilities: vec![],
            dependencies: vec![],
        };

        assert_eq!(metadata.name, "native_test");
    }
}
