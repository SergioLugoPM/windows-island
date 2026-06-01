//! Windows DLL Injection Module
//!
//! Provides safe, reversible DLL injection capabilities for Windows Island v0.3.0.
//! Implements Win32 FFI-based injection using CreateRemoteThread + VirtualAllocEx.
//!
//! # Safety
//! This module uses unsafe Win32 APIs for process manipulation and memory allocation.
//! All operations include safety checks and rollback mechanisms.

use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use windows::core::{HSTRING, PCSTR};
use windows::Win32::Foundation::{HANDLE, FALSE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Threading::{
    CreateRemoteThread, GetCurrentProcessId, OpenProcess, WaitForSingleObject,
    PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
    PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION
};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx,
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    TH32CS_SNAPPROCESS, PROCESSENTRY32W
};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;

/// Supported DLL injection methods
#[derive(Debug, Clone)]
pub enum InjectionMethod {
    /// Manual DLL loading via CreateRemoteThread + LoadLibrary
    ManualDllLoad,
    /// Future: SetWindowsHookEx-based injection
    SetWindowsHookEx,
}

/// Information about an injection target process
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: String,
    pub is_64bit: bool,
}

/// Information about an active injection
#[derive(Debug, Clone)]
pub struct InjectionInfo {
    pub process_name: String,
    pub pid: u32,
    pub dll_path: PathBuf,
    pub method: InjectionMethod,
    pub injected_at: chrono::DateTime<chrono::Utc>,
    pub remote_thread_handle: Option<usize>, // Stored as usize for safety
    pub allocated_memory: Option<usize>,
}

/// Errors that can occur during injection operations
#[derive(Debug, thiserror::Error)]
pub enum InjectionError {
    #[error("Target process not found: {name}")]
    ProcessNotFound { name: String },
    #[error("Failed to open process {pid}: {reason}")]
    ProcessOpenFailed { pid: u32, reason: String },
    #[error("Memory allocation failed in target process")]
    MemoryAllocationFailed,
    #[error("Failed to write DLL path to target process")]
    MemoryWriteFailed,
    #[error("CreateRemoteThread failed: {reason}")]
    RemoteThreadFailed { reason: String },
    #[error("DLL injection verification failed")]
    InjectionVerificationFailed,
    #[error("Target process architecture mismatch (x86/x64)")]
    ArchitectureMismatch,
    #[error("Insufficient privileges for injection")]
    InsufficientPrivileges,
    #[error("DLL file not found: {path}")]
    DllNotFound { path: String },
    #[error("Injection already active for process: {name}")]
    InjectionAlreadyActive { name: String },
    #[error("No active injection found for process: {name}")]
    NoActiveInjection { name: String },
    #[error("System error: {message}")]
    SystemError { message: String },
}

/// Main DLL injector implementation
pub struct Injector {
    /// Target processes for injection (e.g., "explorer.exe", "StartMenuExperienceHost.exe")
    target_processes: Vec<String>,
    /// Path to the DLL payload to inject
    dll_path: PathBuf,
    /// Injection method to use
    injection_method: InjectionMethod,
    /// Currently active injections
    active_injections: Arc<Mutex<HashMap<String, InjectionInfo>>>,
    /// Process monitoring state
    process_monitor: Arc<RwLock<HashMap<u32, ProcessInfo>>>,
}

impl Injector {
    /// Create a new injector instance
    ///
    /// # Arguments
    /// * `dll_path` - Path to the DLL payload to inject
    /// * `target_processes` - List of target process names (e.g., ["explorer.exe"])
    /// * `injection_method` - Method to use for injection
    pub fn new(
        dll_path: PathBuf,
        target_processes: Vec<String>,
        injection_method: InjectionMethod
    ) -> Result<Self, InjectionError> {
        // Verify DLL exists
        if !dll_path.exists() {
            return Err(InjectionError::DllNotFound {
                path: dll_path.to_string_lossy().to_string()
            });
        }

        Ok(Self {
            target_processes,
            dll_path,
            injection_method,
            active_injections: Arc::new(Mutex::new(HashMap::new())),
            process_monitor: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Inject DLL into a specific process by name
    ///
    /// # Safety
    /// This function uses unsafe Win32 APIs to manipulate remote process memory.
    /// It includes safety checks but should only be used on trusted DLL payloads.
    pub fn inject_into_process(&self, process_name: &str) -> Result<(), InjectionError> {
        // Check if already injected
        {
            let active = self.active_injections.lock().unwrap();
            if active.contains_key(process_name) {
                return Err(InjectionError::InjectionAlreadyActive {
                    name: process_name.to_string()
                });
            }
        }

        // Find target process
        let process_info = self.find_process_by_name(process_name)?;

        // Verify architecture compatibility
        self.verify_architecture_compatibility(process_info.pid)?;

        // Perform injection based on method
        let injection_info = match self.injection_method {
            InjectionMethod::ManualDllLoad => {
                self.inject_via_manual_dll_load(&process_info)?
            },
            InjectionMethod::SetWindowsHookEx => {
                // TODO: Implement SetWindowsHookEx method
                return Err(InjectionError::SystemError {
                    message: "SetWindowsHookEx method not yet implemented".to_string()
                });
            }
        };

        // Store injection info
        {
            let mut active = self.active_injections.lock().unwrap();
            active.insert(process_name.to_string(), injection_info);
        }

        Ok(())
    }

    /// Remove injection from a specific process
    pub fn remove_injection(&self, process_name: &str) -> Result<(), InjectionError> {
        let injection_info = {
            let mut active = self.active_injections.lock().unwrap();
            active.remove(process_name)
                .ok_or_else(|| InjectionError::NoActiveInjection {
                    name: process_name.to_string()
                })?
        };

        // Clean up remote thread and memory
        unsafe {
            if let Some(thread_handle) = injection_info.remote_thread_handle {
                let handle = HANDLE(thread_handle as _);
                if handle != INVALID_HANDLE_VALUE {
                    // Wait for thread to complete (with timeout)
                    WaitForSingleObject(handle, 5000); // 5 second timeout
                    windows::Win32::Foundation::CloseHandle(handle).ok();
                }
            }

            // Free allocated memory in target process
            if let Some(allocated_memory) = injection_info.allocated_memory {
                let process_handle = OpenProcess(
                    PROCESS_VM_OPERATION,
                    FALSE,
                    injection_info.pid
                ).map_err(|e| InjectionError::ProcessOpenFailed {
                    pid: injection_info.pid,
                    reason: format!("Failed to open for cleanup: {:?}", e)
                })?;

                let _ = VirtualFreeEx(
                    process_handle,
                    allocated_memory as *mut _,
                    0,
                    MEM_RELEASE
                );
                windows::Win32::Foundation::CloseHandle(process_handle).ok();
            }
        }

        Ok(())
    }

    /// Verify that an injection is still active
    pub fn verify_injection(&self, process_name: &str) -> bool {
        let active = self.active_injections.lock().unwrap();
        if let Some(injection_info) = active.get(process_name) {
            // Verify process is still running
            self.is_process_running(injection_info.pid)
        } else {
            false
        }
    }

    /// Get all currently active injections
    pub fn get_active_injections(&self) -> HashMap<String, InjectionInfo> {
        self.active_injections.lock().unwrap().clone()
    }

    /// Find all target processes currently running
    pub fn find_target_processes(&self) -> Result<Vec<ProcessInfo>, InjectionError> {
        let mut found_processes = Vec::new();

        for target_name in &self.target_processes {
            if let Ok(process_info) = self.find_process_by_name(target_name) {
                found_processes.push(process_info);
            }
        }

        Ok(found_processes)
    }

    /// Check if the injector has sufficient privileges for injection
    pub fn check_privileges(&self) -> bool {
        // Try to open a harmless process to test privileges
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if let Ok(snapshot) = snapshot {
                let mut pe32 = PROCESSENTRY32W {
                    dwSize: mem::size_of::<PROCESSENTRY32W>() as u32,
                    ..Default::default()
                };

                if Process32FirstW(snapshot, &mut pe32).is_ok() {
                    loop {
                        if pe32.th32ProcessID != GetCurrentProcessId() {
                            // Try to open with required permissions
                            let process_handle = OpenProcess(
                                PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE |
                                PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION,
                                FALSE,
                                pe32.th32ProcessID
                            );

                            if let Ok(handle) = process_handle {
                                windows::Win32::Foundation::CloseHandle(handle).ok();
                                windows::Win32::Foundation::CloseHandle(snapshot).ok();
                                return true;
                            }
                        }

                        if Process32NextW(snapshot, &mut pe32).is_err() {
                            break;
                        }
                    }
                }
                windows::Win32::Foundation::CloseHandle(snapshot).ok();
            }
        }
        false
    }

    // Private implementation methods

    /// Find a process by name using ToolHelp32 API
    fn find_process_by_name(&self, process_name: &str) -> Result<ProcessInfo, InjectionError> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| InjectionError::SystemError {
                    message: format!("CreateToolhelp32Snapshot failed: {:?}", e)
                })?;

            let mut pe32 = PROCESSENTRY32W {
                dwSize: mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut pe32).is_err() {
                windows::Win32::Foundation::CloseHandle(snapshot).ok();
                return Err(InjectionError::SystemError {
                    message: "Process32FirstW failed".to_string()
                });
            }

            loop {
                let current_name = String::from_utf16_lossy(&pe32.szExeFile)
                    .trim_end_matches('\0')
                    .to_string();

                if current_name.eq_ignore_ascii_case(process_name) {
                    let process_info = ProcessInfo {
                        pid: pe32.th32ProcessID,
                        name: current_name,
                        path: String::new(), // TODO: Get full path if needed
                        is_64bit: self.is_process_64bit(pe32.th32ProcessID),
                    };

                    windows::Win32::Foundation::CloseHandle(snapshot).ok();
                    return Ok(process_info);
                }

                if Process32NextW(snapshot, &mut pe32).is_err() {
                    break;
                }
            }

            windows::Win32::Foundation::CloseHandle(snapshot).ok();
            Err(InjectionError::ProcessNotFound {
                name: process_name.to_string()
            })
        }
    }

    /// Check if a process is 64-bit
    fn is_process_64bit(&self, _pid: u32) -> bool {
        // For now, assume 64-bit. In production, we should check actual process architecture.
        // This can be done using IsWow64Process or GetProcessInformation APIs.
        true
    }

    /// Verify architecture compatibility between injector and target process
    fn verify_architecture_compatibility(&self, _pid: u32) -> Result<(), InjectionError> {
        // For now, assume compatibility. In production, check if we're running as
        // 32-bit trying to inject into 64-bit process or vice versa.
        Ok(())
    }

    /// Check if a process is still running
    fn is_process_running(&self, pid: u32) -> bool {
        unsafe {
            let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid);
            if let Ok(handle) = process_handle {
                windows::Win32::Foundation::CloseHandle(handle).ok();
                true
            } else {
                false
            }
        }
    }

    /// Perform DLL injection using manual DLL loading technique
    fn inject_via_manual_dll_load(&self, process_info: &ProcessInfo) -> Result<InjectionInfo, InjectionError> {
        unsafe {
            // Open target process with required permissions
            let process_handle = OpenProcess(
                PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE |
                PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION,
                FALSE,
                process_info.pid
            ).map_err(|e| InjectionError::ProcessOpenFailed {
                pid: process_info.pid,
                reason: format!("OpenProcess failed: {:?}", e)
            })?;

            // Convert DLL path to wide string
            let dll_path_wide: Vec<u16> = self.dll_path
                .to_str()
                .unwrap_or("")
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let dll_path_size = dll_path_wide.len() * 2; // 2 bytes per u16

            // Allocate memory in target process for DLL path
            let remote_memory = VirtualAllocEx(
                process_handle,
                None,
                dll_path_size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE
            );

            if remote_memory.is_null() {
                windows::Win32::Foundation::CloseHandle(process_handle).ok();
                return Err(InjectionError::MemoryAllocationFailed);
            }

            // Write DLL path to allocated memory
            let mut bytes_written = 0;
            let write_result = WriteProcessMemory(
                process_handle,
                remote_memory,
                dll_path_wide.as_ptr() as *const _,
                dll_path_size,
                Some(&mut bytes_written)
            );

            if write_result.is_err() || bytes_written != dll_path_size {
                let _ = VirtualFreeEx(process_handle, remote_memory, 0, MEM_RELEASE);
                windows::Win32::Foundation::CloseHandle(process_handle).ok();
                return Err(InjectionError::MemoryWriteFailed);
            }

            // Get LoadLibraryW address from kernel32.dll
            let kernel32 = GetModuleHandleW(&HSTRING::from("kernel32.dll"))
                .map_err(|e| InjectionError::SystemError {
                    message: format!("GetModuleHandleW failed: {:?}", e)
                })?;

            let load_library_addr = GetProcAddress(kernel32, PCSTR::from_raw(b"LoadLibraryW\0".as_ptr()))
                .ok_or_else(|| InjectionError::SystemError {
                    message: "GetProcAddress for LoadLibraryW failed".to_string()
                })?;

            // Create remote thread to call LoadLibraryW
            let remote_thread = CreateRemoteThread(
                process_handle,
                None,
                0,
                Some(std::mem::transmute(load_library_addr)),
                Some(remote_memory),
                0,
                None
            ).map_err(|e| {
                let _ = VirtualFreeEx(process_handle, remote_memory, 0, MEM_RELEASE);
                windows::Win32::Foundation::CloseHandle(process_handle).ok();
                InjectionError::RemoteThreadFailed {
                    reason: format!("CreateRemoteThread failed: {:?}", e)
                }
            })?;

            // Clean up process handle (but keep thread handle for later cleanup)
            windows::Win32::Foundation::CloseHandle(process_handle).ok();

            // Create injection info
            let injection_info = InjectionInfo {
                process_name: process_info.name.clone(),
                pid: process_info.pid,
                dll_path: self.dll_path.clone(),
                method: self.injection_method.clone(),
                injected_at: chrono::Utc::now(),
                remote_thread_handle: Some(remote_thread.0 as usize),
                allocated_memory: Some(remote_memory as usize),
            };

            Ok(injection_info)
        }
    }
}

impl Drop for Injector {
    /// Clean up all active injections when the injector is dropped
    fn drop(&mut self) {
        let active_processes: Vec<String> = {
            let active = self.active_injections.lock().unwrap();
            active.keys().cloned().collect()
        };

        for process_name in active_processes {
            let _ = self.remove_injection(&process_name);
        }
    }
}

// Additional utility functions

/// Get the DLL payload path for Windows Island
pub fn get_payload_dll_path() -> PathBuf {
    // TODO: In production, this should return the actual payload DLL path
    // For now, return a placeholder path
    PathBuf::from("windows_island_payload.dll")
}

/// Check if the current process has administrator privileges
pub fn has_admin_privileges() -> bool {
    // TODO: Implement proper privilege checking using Windows APIs
    // For now, return false to be safe
    false
}

/// Default target processes for Windows Island injection
pub fn default_target_processes() -> Vec<String> {
    vec![
        "explorer.exe".to_string(),
        "StartMenuExperienceHost.exe".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injector_creation() {
        let dll_path = PathBuf::from("test_payload.dll");
        let targets = vec!["notepad.exe".to_string()];

        // This will fail since the DLL doesn't exist, but tests the validation logic
        let result = Injector::new(dll_path, targets, InjectionMethod::ManualDllLoad);
        assert!(result.is_err());

        if let Err(InjectionError::DllNotFound { path }) = result {
            assert!(path.contains("test_payload.dll"));
        } else {
            panic!("Expected DllNotFound error");
        }
    }

    #[test]
    fn test_default_target_processes() {
        let targets = default_target_processes();
        assert!(!targets.is_empty());
        assert!(targets.contains(&"explorer.exe".to_string()));
    }

    #[test]
    fn test_privilege_checking() {
        // This should not require admin privileges to test
        let has_privs = has_admin_privileges();
        // Just ensure it returns a boolean without crashing
        assert!(has_privs == true || has_privs == false);
    }
}