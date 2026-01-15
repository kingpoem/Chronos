//! Batch loader for user programs
//! 
//! Loads user programs from embedded binaries via link_app.S

use super::task::TaskControlBlock;
use crate::task::TASK_MANAGER;

// External symbols from link_app.S (manually maintained)
extern "C" {
    fn _num_app();
}

/// Get number of apps
fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// Get app data by index
fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    crate::println!("[Loader] app_start array: {:?}", app_start);
    assert!(app_id < num_app, "App ID {} out of range (max: {})", app_id, num_app);
    let start = app_start[app_id];
    let end = app_start[app_id + 1];
    crate::println!("[Loader] app {} range: {:#x} - {:#x}", app_id, start, end);
    unsafe {
        core::slice::from_raw_parts(
            start as *const u8,
            end - start,
        )
    }
}

/// Load user programs in batch
pub fn load_apps() {
    crate::println!("[Loader] Loading user programs...");
    
    let num_app = get_num_app();
    crate::println!("[Loader] Found {} user programs", num_app);
    
    for i in 0..num_app {
        let app_data = get_app_data(i);
        if app_data.is_empty() {
            crate::println!("[Loader] Warning: App {} is empty, skipping", i);
            continue;
        }
        
        crate::println!("[Loader] Loading app {} (size: {} bytes)", i, app_data.len());
        
        let task = TaskControlBlock::new(app_data, i);
        let pid = TASK_MANAGER.lock().add_task(task);
        crate::println!("[Loader] Loaded app {} as task {}", i, pid);
    }
    
    crate::println!("[Loader] Loaded {} programs", TASK_MANAGER.lock().task_count());
}
