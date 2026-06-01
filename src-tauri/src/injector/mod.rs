//! DLL injector for Theme injection into Explorer.exe and StartMenuExperienceHost.exe
//!
//! Phase 1: Basic injection pipeline with shared memory theme IPC.
//! No rendering hooks yet — logs only.

use std::ffi::CString;
use std::path::PathBuf;
use std::ptr;
use windows::Win32::System::ProcessStatus::K32EnumProcesses;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::System::Memory::{VirtualAllocEx, WriteProcessMemory, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE};
use windows::Win32::System::Threading::{CreateRemoteThread, WaitForSingleObject, GetCurrentProcess, OpenProcess, CloseHandle, PROCESS_ALL_ACCESS};

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
        self.inject_into_process("explorer.exe")
    }

    /// Inject DLL into StartMenuExperienceHost.exe (Win11 Start Menu)
    pub fn inject_into_startmenu(&self) -> Result<(), InjectorError> {
        self.inject_into_process("StartMenuExperienceHost.exe")
    }

    fn inject_into_process(&self, process_name: &str) -> Result<(), InjectorError> {
        let pid = self.find_process_by_name(process_name)?;
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

            if Process32First(snapshot, &mut entry).as_bool() {
                loop {
                    let exe_name = std::ffi::CStr::from_ptr(entry.szExeFile.as_ptr() as *const i8)
                        .to_string_lossy();
                    if exe_name.eq_ignore_ascii_case(name) {
                        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
                        return Ok(entry.th32ProcessID);
                    }
                    if !Process32Next(snapshot, &mut entry).as_bool() {
                        break;
                    }
                }
            }
            let _ = windows::Win32::Foundation::CloseHandle(snapshot);
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
            let remote_mem = VirtualAllocEx(h_process, None, dll_bytes.len(), MEM_COMMIT, PAGE_READWRITE)
                .ok_or(InjectorError::AllocFailed)?;

            // Write DLL path to target process
            let mut bytes_written = 0;
            WriteProcessMemory(h_process, remote_mem, dll_bytes.as_ptr() as *const _, dll_bytes.len(), Some(&mut bytes_written))
                .map_err(|_| InjectorError::WriteFailed)?;

            if bytes_written != dll_bytes.len() {
                let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
                let _ = CloseHandle(h_process);
                return Err(InjectorError::WriteFailed);
            }

            // Get LoadLibraryA address
            let load_library_a = windows::Win32::System::LibraryLoader::LoadLibraryA as *mut ();

            // Create remote thread to call LoadLibraryA(remote_mem)
            let h_thread = CreateRemoteThread(
                h_process,
                None,
                0,
                Some(std::mem::transmute(load_library_a)),
                Some(remote_mem),
                0,
                None,
            );

            if h_thread.is_invalid() {
                let _ = VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE);
                let _ = CloseHandle(h_process);
                return Err(InjectorError::CreateThreadFailed);
            }

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
