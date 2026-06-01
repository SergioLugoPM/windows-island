//! DLL injector for Theme injection into Explorer.exe and StartMenuExperienceHost.exe
//!
//! Phase 1: Basic injection pipeline with shared memory theme IPC.
//! No rendering hooks yet — logs only.

use std::ffi::CString;
use std::path::PathBuf;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Memory::{VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::Threading::{CreateRemoteThread, WaitForSingleObject, OpenProcess, PROCESS_ALL_ACCESS};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

pub struct Injector {
    dll_path: PathBuf,
}

#[derive(Debug)]
pub enum InjectorError {
    ProcessNotFound(String),
    OpenProcessFailed(String),
    AllocFailed,
    WriteFailed,
    CreateThreadFailed,
    DllNotFound,
}

impl Injector {
    pub fn new(dll_path: PathBuf) -> Self {
        Self { dll_path }
    }

    /// Inject DLL into Explorer.exe (taskbar)
    pub fn inject_into_explorer(&self) -> Result<(), InjectorError> {
        let pid = self.find_process_by_name("explorer.exe")?;
        self.inject_into_pid(pid)
    }

    /// Inject DLL into StartMenuExperienceHost.exe (Win11 Start Menu)
    pub fn inject_into_startmenu(&self) -> Result<(), InjectorError> {
        let pid = self.find_process_by_name("StartMenuExperienceHost.exe")?;
        self.inject_into_pid(pid)
    }

    fn find_process_by_name(&self, name: &str) -> Result<u32, InjectorError> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|_| InjectorError::ProcessNotFound(name.to_string()))?;

            let mut entry = PROCESSENTRY32 {
                dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
                ..Default::default()
            };

            if Process32First(snapshot, &mut entry).is_ok() {
                loop {
                    let exe_name = std::ffi::CStr::from_ptr(entry.szExeFile.as_ptr() as *const i8)
                        .to_string_lossy();
                    if exe_name.eq_ignore_ascii_case(name) {
                        let _ = CloseHandle(snapshot);
                        return Ok(entry.th32ProcessID);
                    }
                    if Process32Next(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        Err(InjectorError::ProcessNotFound(name.to_string()))
    }

    fn inject_into_pid(&self, pid: u32) -> Result<(), InjectorError> {
        if !self.dll_path.exists() {
            return Err(InjectorError::DllNotFound);
        }

        unsafe {
            // Open target process
            let h_process = OpenProcess(PROCESS_ALL_ACCESS, false, pid)
                .map_err(|_| InjectorError::OpenProcessFailed(format!("PID {}", pid)))?;

            // Get DLL path as ANSI C string for LoadLibraryA
            let dll_path_str = self.dll_path.to_string_lossy();
            let dll_cstring = CString::new(dll_path_str.as_bytes())
                .map_err(|_| InjectorError::AllocFailed)?;
            let dll_bytes = dll_cstring.as_bytes_with_nul();

            // Allocate memory in target process
            let remote_mem = VirtualAllocEx(h_process, None, dll_bytes.len(), MEM_COMMIT, PAGE_READWRITE);
            if remote_mem.is_null() {
                let _ = CloseHandle(h_process);
                return Err(InjectorError::AllocFailed);
            }

            // Write DLL path to target process
            let mut bytes_written = 0;
            if let Err(_) = WriteProcessMemory(h_process, remote_mem, dll_bytes.as_ptr() as *const _, dll_bytes.len(), Some(&mut bytes_written)) {
                let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
                let _ = CloseHandle(h_process);
                return Err(InjectorError::WriteFailed);
            }

            if bytes_written != dll_bytes.len() {
                let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
                let _ = CloseHandle(h_process);
                return Err(InjectorError::WriteFailed);
            }

            // Get LoadLibraryA address from kernel32.dll
            let kernel32 = match GetModuleHandleW(windows::core::w!("kernel32.dll")) {
                Ok(h) => h,
                Err(_) => {
                    let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
                    let _ = CloseHandle(h_process);
                    return Err(InjectorError::CreateThreadFailed);
                }
            };
            let load_library_a = match GetProcAddress(kernel32, windows::core::s!("LoadLibraryA")) {
                Some(addr) => addr as *mut (),
                None => {
                    let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
                    let _ = CloseHandle(h_process);
                    return Err(InjectorError::CreateThreadFailed);
                }
            };

            // Create remote thread to call LoadLibraryA(remote_mem)
            let h_thread = match CreateRemoteThread(
                h_process,
                None,
                0,
                Some(std::mem::transmute(load_library_a)),
                Some(remote_mem),
                0,
                None,
            ) {
                Ok(handle) => handle,
                Err(_) => {
                    let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
                    let _ = CloseHandle(h_process);
                    return Err(InjectorError::CreateThreadFailed);
                }
            };

            // Wait for thread to complete (max 5 seconds)
            let _ = WaitForSingleObject(h_thread, 5000);

            // Cleanup
            let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
            let _ = CloseHandle(h_thread);
            let _ = CloseHandle(h_process);
        }
        Ok(())
    }
}
