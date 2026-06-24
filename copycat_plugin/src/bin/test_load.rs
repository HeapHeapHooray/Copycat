use std::path::Path;
use std::ffi::{c_char, c_void, CStr, CString};

#[repr(C)]
pub struct clap_plugin_entry {
    pub clap_version: [u32; 3],
    pub init: unsafe extern "C" fn(plugin_path: *const c_char) -> bool,
    pub deinit: unsafe extern "C" fn(),
    pub get_factory: unsafe extern "C" fn(factory_id: *const c_char) -> *const c_void,
}

#[repr(C)]
pub struct clap_plugin_factory {
    pub get_plugin_count: unsafe extern "C" fn(factory: *const c_void) -> u32,
    pub get_plugin_descriptor: unsafe extern "C" fn(
        factory: *const c_void,
        index: u32,
    ) -> *const clap_plugin_descriptor,
    pub create_plugin: unsafe extern "C" fn(
        factory: *const c_void,
        host: *const clap_host,
        plugin_id: *const c_char,
    ) -> *const clap_plugin,
}

#[repr(C)]
pub struct clap_plugin_descriptor {
    pub clap_version: [u32; 3],
    pub id: *const c_char,
    pub name: *const c_char,
    pub vendor: *const c_char,
    pub url: *const c_char,
    pub manual_url: *const c_char,
    pub support_url: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub features: *const *const c_char,
}

#[repr(C)]
pub struct clap_host {
    pub clap_version: [u32; 3],
    pub host_data: *mut c_void,
    pub name: *const c_char,
    pub vendor: *const c_char,
    pub version: *const c_char,
    pub get_extension: unsafe extern "C" fn(host: *const clap_host, extension_id: *const c_char) -> *const c_void,
    pub request_restart: unsafe extern "C" fn(host: *const clap_host),
    pub request_process: unsafe extern "C" fn(host: *const clap_host),
    pub request_callback: unsafe extern "C" fn(host: *const clap_host),
}

#[repr(C)]
pub struct clap_plugin {
    pub desc: *const clap_plugin_descriptor,
    pub plugin_data: *mut c_void,
    pub init: unsafe extern "C" fn(plugin: *const clap_plugin) -> bool,
    pub destroy: unsafe extern "C" fn(plugin: *const clap_plugin),
    pub activate: unsafe extern "C" fn(
        plugin: *const clap_plugin,
        sample_rate: f64,
        min_frames_count: u32,
        max_frames_count: u32,
    ) -> bool,
    pub deactivate: unsafe extern "C" fn(plugin: *const clap_plugin),
    pub start_processing: unsafe extern "C" fn(plugin: *const clap_plugin) -> bool,
    pub stop_processing: unsafe extern "C" fn(plugin: *const clap_plugin),
    pub reset: unsafe extern "C" fn(plugin: *const clap_plugin),
    pub process: unsafe extern "C" fn(
        plugin: *const clap_plugin,
        process: *const c_void,
    ),
    pub get_extension: unsafe extern "C" fn(
        plugin: *const clap_plugin,
        id: *const c_char,
    ) -> *const c_void,
    pub on_main_thread: unsafe extern "C" fn(plugin: *const clap_plugin),
}

#[repr(C)]
pub struct clap_window {
    pub api: *const c_char,
    pub specific: clap_window_specific,
}

#[repr(C)]
pub union clap_window_specific {
    pub cocoa: *mut c_void,
    pub x11: std::os::raw::c_ulong,
    pub win32: *mut c_void,
    pub ptr: *mut c_void,
}

#[repr(C)]
pub struct clap_plugin_gui {
    pub is_api_supported: unsafe extern "C" fn(plugin: *const clap_plugin, api: *const c_char, is_floating: bool) -> bool,
    pub get_preferred_api: unsafe extern "C" fn(plugin: *const clap_plugin, api: *mut *const c_char, is_floating: *mut bool) -> bool,
    pub create: unsafe extern "C" fn(plugin: *const clap_plugin, api: *const c_char, is_floating: bool) -> bool,
    pub destroy: unsafe extern "C" fn(plugin: *const clap_plugin),
    pub set_scale: unsafe extern "C" fn(plugin: *const clap_plugin, scale: f64) -> bool,
    pub get_size: unsafe extern "C" fn(plugin: *const clap_plugin, width: *mut u32, height: *mut u32) -> bool,
    pub can_resize: unsafe extern "C" fn(plugin: *const clap_plugin) -> bool,
    pub get_resize_hints: unsafe extern "C" fn(plugin: *const clap_plugin, hints: *mut c_void) -> bool,
    pub adjust_size: unsafe extern "C" fn(plugin: *const clap_plugin, width: *mut u32, height: *mut u32) -> bool,
    pub set_size: unsafe extern "C" fn(plugin: *const clap_plugin, width: u32, height: u32) -> bool,
    pub set_parent: unsafe extern "C" fn(plugin: *const clap_plugin, window: *const clap_window) -> bool,
    pub set_transient: unsafe extern "C" fn(plugin: *const clap_plugin, window: *const clap_window) -> bool,
    pub suggest_title: unsafe extern "C" fn(plugin: *const clap_plugin, title: *const c_char),
    pub show: unsafe extern "C" fn(plugin: *const clap_plugin) -> bool,
    pub hide: unsafe extern "C" fn(plugin: *const clap_plugin) -> bool,
}

#[link(name = "user32")]
extern "system" {
    pub fn CreateWindowExA(
        dwExStyle: u32,
        lpClassName: *const c_char,
        lpWindowName: *const c_char,
        dwStyle: u32,
        x: i32,
        y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: *mut c_void,
        hMenu: *mut c_void,
        hInstance: *mut c_void,
        lpParam: *mut c_void,
    ) -> *mut c_void;
    pub fn DestroyWindow(hWnd: *mut c_void) -> bool;
}

unsafe extern "C" fn dummy_get_extension(_host: *const clap_host, _extension_id: *const c_char) -> *const c_void {
    std::ptr::null()
}

unsafe extern "C" fn dummy_request(_host: *const clap_host) {}

fn main() {
    println!("=== Copycat CLAP Instantiation Test ===");
    
    let path_str = "C:\\Program Files\\Common Files\\CLAP\\copycat_new.clap";
    let clap_path = Path::new(path_str);
    if !clap_path.exists() {
        println!("Error: CLAP plugin not found at {:?}", clap_path);
        return;
    }
    
    println!("Loading CLAP library...");
    let lib = match unsafe { libloading::Library::new(clap_path) } {
        Ok(l) => l,
        Err(e) => {
            println!("ERROR: Failed to load CLAP library: {:?}", e);
            return;
        }
    };
    
    println!("Getting clap_entry symbol...");
    let entry_ptr: libloading::Symbol<*const clap_plugin_entry> = match unsafe { lib.get(b"clap_entry") } {
        Ok(sym) => sym,
        Err(e) => {
            println!("ERROR: Failed to get clap_entry: {:?}", e);
            return;
        }
    };
    
    let entry = unsafe { &**entry_ptr };
    println!("CLAP version: {}.{}.{}", entry.clap_version[0], entry.clap_version[1], entry.clap_version[2]);
    
    println!("Initializing entry...");
    let c_path = CString::new(path_str).unwrap();
    if unsafe { (entry.init)(c_path.as_ptr()) } {
        println!("SUCCESS: entry initialized successfully!");
    } else {
        println!("ERROR: entry initialization failed!");
        return;
    }
    
    println!("Getting CLAP factory...");
    let factory_id = CString::new("clap.plugin-factory").unwrap();
    let factory_void = unsafe { (entry.get_factory)(factory_id.as_ptr()) };
    if factory_void.is_null() {
        println!("ERROR: Failed to get plugin factory!");
        return;
    }
    
    let factory = unsafe { &*(factory_void as *const clap_plugin_factory) };
    let count = unsafe { (factory.get_plugin_count)(factory_void) };
    println!("Found {} plugins in factory.", count);
    
    if count == 0 {
        println!("No plugins found. Exiting.");
        return;
    }
    
    let desc = unsafe { &*(factory.get_plugin_descriptor)(factory_void, 0) };
    let id_str = unsafe { CStr::from_ptr(desc.id).to_string_lossy().into_owned() };
    let name_str = unsafe { CStr::from_ptr(desc.name).to_string_lossy().into_owned() };
    println!("Plugin 0 ID: {}, Name: {}", id_str, name_str);
    
    // Create mock host
    let host_name = CString::new("TestHost").unwrap();
    let mock_host = clap_host {
        clap_version: [1, 2, 0],
        host_data: std::ptr::null_mut(),
        name: host_name.as_ptr(),
        vendor: host_name.as_ptr(),
        version: host_name.as_ptr(),
        get_extension: dummy_get_extension,
        request_restart: dummy_request,
        request_process: dummy_request,
        request_callback: dummy_request,
    };
    
    println!("Instantiating CLAP plugin: {}...", id_str);
    let c_id = CString::new(id_str).unwrap();
    let plugin_ptr = unsafe { (factory.create_plugin)(factory_void, &mock_host, c_id.as_ptr()) };
    
    if plugin_ptr.is_null() {
        println!("ERROR: Failed to create CLAP plugin instance!");
        return;
    }
    
    println!("SUCCESS: CLAP plugin instance created at address: {:p}", plugin_ptr);
    
    let plugin = unsafe { &*plugin_ptr };
    println!("Initializing CLAP plugin instance...");
    if unsafe { (plugin.init)(plugin_ptr) } {
        println!("SUCCESS: CLAP plugin instance initialized!");
    } else {
        println!("ERROR: CLAP plugin instance initialization failed!");
        return;
    }

    println!("Activating CLAP plugin instance...");
    if unsafe { (plugin.activate)(plugin_ptr, 44100.0, 32, 1024) } {
        println!("SUCCESS: CLAP plugin instance activated!");
    } else {
        println!("ERROR: CLAP plugin instance activation failed!");
    }

    println!("Querying CLAP GUI extension...");
    let gui_ext_id = CString::new("clap.gui").unwrap();
    let gui_void = unsafe { (plugin.get_extension)(plugin_ptr, gui_ext_id.as_ptr()) };
    if gui_void.is_null() {
        println!("INFO: Plugin does not support clap.gui extension.");
    } else {
        println!("SUCCESS: Found clap.gui extension at {:p}", gui_void);
        let gui = unsafe { &*(gui_void as *const clap_plugin_gui) };
        
        let mut api_ptr: *const c_char = std::ptr::null();
        let mut is_floating: bool = false;
        let mut preferred_api = String::new();
        if unsafe { (gui.get_preferred_api)(plugin_ptr, &mut api_ptr, &mut is_floating) } && !api_ptr.is_null() {
            let api_str = unsafe { CStr::from_ptr(api_ptr).to_string_lossy().into_owned() };
            println!("Preferred GUI API: {}, floating: {}", api_str, is_floating);
            preferred_api = api_str;
        } else {
            println!("get_preferred_api failed or returned null!");
        }

        if !preferred_api.is_empty() {
            let api_id = CString::new(preferred_api.clone()).unwrap();
            println!("Checking if {} GUI API is supported...", preferred_api);
            let supported = unsafe { (gui.is_api_supported)(plugin_ptr, api_id.as_ptr(), false) };
            println!("Supported: {}", supported);

            if supported {
                println!("Creating GUI window using {}...", preferred_api);
                let created = unsafe { (gui.create)(plugin_ptr, api_id.as_ptr(), false) };
                if created {
                    println!("SUCCESS: GUI window created!");
                    
                    // Create a dummy Win32 window to act as parent
                    let class_name = CString::new("Static").unwrap();
                    let window_name = CString::new("Parent").unwrap();
                    let hwnd = unsafe {
                        CreateWindowExA(
                            0,
                            class_name.as_ptr(),
                            window_name.as_ptr(),
                            0,
                            0, 0, 100, 100,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            std::ptr::null_mut()
                        )
                    };
                    
                    if hwnd.is_null() {
                        println!("ERROR: Failed to create Win32 parent window!");
                    } else {
                        println!("SUCCESS: Created Win32 parent window at {:p}", hwnd);
                        
                        let clap_win = clap_window {
                            api: api_id.as_ptr(),
                            specific: clap_window_specific { win32: hwnd },
                        };
                        
                        println!("Calling set_parent on GUI...");
                        let parented = unsafe { (gui.set_parent)(plugin_ptr, &clap_win) };
                        println!("set_parent result: {}", parented);
                        
                        if parented {
                            println!("Calling show on GUI...");
                            let shown = unsafe { (gui.show)(plugin_ptr) };
                            println!("show result: {}", shown);
                            
                            println!("Calling hide on GUI...");
                            unsafe { (gui.hide)(plugin_ptr) };
                        }
                        
                        // Clean up window
                        unsafe { DestroyWindow(hwnd) };
                    }
                    
                    println!("Destroying GUI window...");
                    unsafe { (gui.destroy)(plugin_ptr) };
                    println!("SUCCESS: GUI window destroyed!");
                } else {
                    println!("ERROR: GUI window creation failed!");
                }
            }
        }
    }

    println!("Deactivating CLAP plugin instance...");
    unsafe { (plugin.deactivate)(plugin_ptr) };
    
    println!("Destroying CLAP plugin instance...");
    unsafe { (plugin.destroy)(plugin_ptr) };
    println!("SUCCESS: CLAP plugin instance destroyed!");
    
    println!("Deinitializing entry...");
    unsafe { (entry.deinit)() };
    println!("=== Test Completed ===");
}
