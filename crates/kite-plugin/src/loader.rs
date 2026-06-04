use kite_core::plugin::{Plugin, PluginError};
use libloading::{Library, Symbol};
use std::path::Path;

type PluginCreateFn = unsafe fn() -> *mut dyn Plugin;

pub struct NativePlugin {
    _library: Library,
    pub instance: Box<dyn Plugin>,
}

impl NativePlugin {
    /// # Safety
    /// The shared library at `path` must export a valid `_kite_plugin_create` function
    /// that returns a properly constructed Plugin trait object.
    pub unsafe fn load(path: &Path) -> Result<Self, PluginError> {
        let library = Library::new(path).map_err(|e| {
            PluginError::InitFailed(format!("Failed to load library {:?}: {}", path, e))
        })?;

        let create: Symbol<PluginCreateFn> =
            library.get(b"_kite_plugin_create").map_err(|e| {
                PluginError::InitFailed(format!("Symbol not found: {}", e))
            })?;

        let raw = create();
        let instance = Box::from_raw(raw);

        Ok(Self {
            _library: library,
            instance,
        })
    }
}

/// Macro for plugin authors to export the required symbols.
/// Usage in plugin crate: `kite_plugin::export_plugin!(MyPluginStruct);`
#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn _kite_plugin_create() -> *mut dyn kite_core::plugin::Plugin {
            let plugin: $plugin_type = Default::default();
            Box::into_raw(Box::new(plugin))
        }

        #[no_mangle]
        pub extern "C" fn _kite_plugin_destroy(ptr: *mut dyn kite_core::plugin::Plugin) {
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    };
}
