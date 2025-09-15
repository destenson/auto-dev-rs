// WASM Module Host
//
// Provides sandboxed execution environment for WebAssembly modules

use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasmtime::{Engine, Module, Store, Instance, Linker, Memory, Func};
use async_trait::async_trait;

use crate::modules::interface::{
    ModuleInterface, ModuleMetadata, ModuleCapability, ModuleState,
    ModuleVersion, ModuleDependency, ResourceLimits
};

/// WASM module wrapper
#[derive(Clone)]
pub struct WasmModule {
    metadata: ModuleMetadata,
    module: Arc<Module>,
    engine: Arc<Engine>,
    state: ModuleState,
    resource_limits: ResourceLimits,
}

impl WasmModule {
    /// Load a WASM module from file
    pub async fn load(path: &Path, engine: &Arc<Engine>) -> Result<Self> {
        let wasm_bytes = tokio::fs::read(path).await
            .context("Failed to read WASM file")?;

        let module = Module::new(engine, &wasm_bytes)
            .context("Failed to compile WASM module")?;

        // Extract metadata from the module
        // In a real implementation, this would be extracted from the WASM module's custom section
        let metadata = Self::extract_metadata(&module)?;

        let state = ModuleState::new(metadata.version.clone());

        Ok(Self {
            metadata,
            module: Arc::new(module),
            engine: engine.clone(),
            state,
            resource_limits: ResourceLimits::default(),
        })
    }

    /// Extract metadata from WASM module
    fn extract_metadata(module: &Module) -> Result<ModuleMetadata> {
        // In a real implementation, we would read this from a custom section in the WASM file
        // For now, return placeholder metadata
        Ok(ModuleMetadata {
            name: "wasm_module".to_string(),
            version: ModuleVersion::new(1, 0, 0),
            author: "auto-dev".to_string(),
            description: "WebAssembly module".to_string(),
            capabilities: vec![],
            dependencies: vec![],
        })
    }

    /// Create a new store with configured limits
    fn create_store(&self) -> Result<Store<WasmState>> {
        let mut store = Store::new(&self.engine, WasmState::default());
        
        // Configure resource limits
        store.limiter(|state| &mut state.limits);
        
        // Set fuel for CPU time limiting
        // Note: fuel limiting would be configured at engine level with consume_fuel
        
        Ok(store)
    }

    /// Setup host functions that modules can call
    fn setup_host_functions(&self, linker: &mut Linker<WasmState>) -> Result<()> {
        // Console log function
        linker.func_wrap("env", "console_log", |mut caller: wasmtime::Caller<'_, WasmState>, ptr: i32, len: i32| {
            let memory = caller.get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| anyhow::anyhow!("Failed to get memory"))?;
            
            let data = memory.data(&caller);
            let msg = std::str::from_utf8(&data[ptr as usize..(ptr + len) as usize])
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))?;
            
            tracing::info!("WASM module log: {}", msg);
            Ok(())
        })?;

        // Get timestamp function
        linker.func_wrap("env", "get_timestamp", || -> i64 {
            chrono::Utc::now().timestamp_millis()
        })?;

        // Allocate memory function
        linker.func_wrap("env", "allocate", |mut caller: wasmtime::Caller<'_, WasmState>, size: i32| -> Result<i32> {
            // Check if allocation is within limits
            let state = caller.data_mut();
            state.memory_used += size as u64;
            
            if state.memory_used > 100 * 1024 * 1024 { // 100MB limit
                return Err(anyhow::anyhow!("Memory limit exceeded"));
            }
            
            // In a real implementation, we would call the module's allocator
            Ok(0) // Return pointer to allocated memory
        })?;

        // Deallocate memory function
        linker.func_wrap("env", "deallocate", |mut caller: wasmtime::Caller<'_, WasmState>, _ptr: i32, size: i32| {
            let state = caller.data_mut();
            state.memory_used = state.memory_used.saturating_sub(size as u64);
        })?;

        Ok(())
    }

    /// Execute a function in the WASM module
    async fn call_function(&self, func_name: &str, input: &[u8]) -> Result<Vec<u8>> {
        let mut store = self.create_store()?;
        let mut linker = Linker::new(&self.engine);
        
        // Setup host functions
        self.setup_host_functions(&mut linker)?;
        
        // Instantiate the module
        let instance = linker.instantiate(&mut store, &self.module)
            .context("Failed to instantiate WASM module")?;
        
        // Get memory export
        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("Module doesn't export memory"))?;
        
        // Get the function to call
        let func = instance.get_func(&mut store, func_name)
            .ok_or_else(|| anyhow::anyhow!("Function not found: {}", func_name))?;
        
        // Write input to memory
        let input_ptr = self.write_to_memory(&mut store, &memory, input)?;
        
        // Call the function
        let mut results = vec![wasmtime::Val::I32(0)];
        func.call(&mut store, &[wasmtime::Val::I32(input_ptr), wasmtime::Val::I32(input.len() as i32)], &mut results)
            .context("Failed to call WASM function")?;
        
        // Read output from memory
        let output_ptr = results[0].unwrap_i32();
        let output_len = self.get_output_length(&mut store, &instance)?;
        
        self.read_from_memory(&store, &memory, output_ptr, output_len)
    }

    /// Write data to WASM memory
    fn write_to_memory(&self, store: &mut Store<WasmState>, memory: &Memory, data: &[u8]) -> Result<i32> {
        let mem_data = memory.data_mut(store);
        
        // Simple allocation at offset 0 for demo
        // In production, use proper memory management
        let ptr = 0;
        mem_data[ptr..ptr + data.len()].copy_from_slice(data);
        
        Ok(ptr as i32)
    }

    /// Read data from WASM memory
    fn read_from_memory(&self, store: &Store<WasmState>, memory: &Memory, ptr: i32, len: i32) -> Result<Vec<u8>> {
        let data = memory.data(store);
        let start = ptr as usize;
        let end = start + len as usize;
        
        if end > data.len() {
            anyhow::bail!("Memory read out of bounds");
        }
        
        Ok(data[start..end].to_vec())
    }

    /// Get output length from module
    fn get_output_length(&self, store: &mut Store<WasmState>, instance: &Instance) -> Result<i32> {
        // Call get_output_length function if it exists
        if let Some(func) = instance.get_func(&mut *store, "get_output_length") {
            let results = {
                let mut results = vec![wasmtime::Val::I32(0)];
                func.call(&mut *store, &[], &mut results)?;
                results
            };
            Ok(results[0].unwrap_i32())
        } else {
            // Default output length
            Ok(1024)
        }
    }
}

/// State for WASM execution
#[derive(Default)]
struct WasmState {
    limits: StoreLimits,
    memory_used: u64,
}

/// Resource limiter for WASM execution
struct StoreLimits;

impl Default for StoreLimits {
    fn default() -> Self {
        StoreLimits
    }
}

impl wasmtime::ResourceLimiter for StoreLimits {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
        // Limit to 100MB
        Ok(desired <= 100 * 1024 * 1024)
    }

    fn table_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
        // Limit tables to 10000 elements
        Ok(desired <= 10000)
    }
}

#[async_trait]
impl ModuleInterface for WasmModule {
    fn metadata(&self) -> ModuleMetadata {
        self.metadata.clone()
    }

    async fn initialize(&mut self) -> Result<()> {
        // Call initialization function if it exists
        match self.call_function("initialize", &[]).await {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("Function not found") => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let input_bytes = serde_json::to_vec(&input)?;
        let output_bytes = self.call_function("execute", &input_bytes).await?;
        let output: Value = serde_json::from_slice(&output_bytes)?;
        Ok(output)
    }

    fn get_capabilities(&self) -> Vec<ModuleCapability> {
        self.metadata.capabilities.clone()
    }

    async fn handle_message(&mut self, message: Value) -> Result<Option<Value>> {
        let input_bytes = serde_json::to_vec(&message)?;
        
        match self.call_function("handle_message", &input_bytes).await {
            Ok(output_bytes) => {
                if output_bytes.is_empty() {
                    Ok(None)
                } else {
                    let output: Value = serde_json::from_slice(&output_bytes)?;
                    Ok(Some(output))
                }
            }
            Err(e) if e.to_string().contains("Function not found") => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Call shutdown function if it exists
        match self.call_function("shutdown", &[]).await {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("Function not found") => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn get_state(&self) -> Result<ModuleState> {
        Ok(self.state.clone())
    }

    fn restore_state(&mut self, state: ModuleState) -> Result<()> {
        self.state = state;
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        match self.call_function("health_check", &[]).await {
            Ok(result) => {
                let healthy: bool = serde_json::from_slice(&result).unwrap_or(true);
                Ok(healthy)
            }
            Err(e) if e.to_string().contains("Function not found") => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wasm_module_metadata() {
        let metadata = ModuleMetadata {
            name: "test".to_string(),
            version: ModuleVersion::new(1, 0, 0),
            author: "test".to_string(),
            description: "test module".to_string(),
            capabilities: vec![],
            dependencies: vec![],
        };

        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.version.major, 1);
    }
}
