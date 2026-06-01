//! PE header parser for IAT patching.
//!
//! Walks the Import Address Table (IAT) of the calling process's main module
//! (Explorer.exe when injected) to locate a named import and replace it with
//! a hook function pointer.
//!
//! All structs use `#[repr(C)]` with field layouts that match the Microsoft
//! PE/COFF specification exactly (x86-64 / PE32+ only).

use std::ffi::CStr;
use std::mem::size_of;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE};

// ── Raw PE structures ─────────────────────────────────────────────────────────

/// IMAGE_DOS_HEADER — only `e_magic` (offset 0) and `e_lfanew` (offset 60)
/// are accessed; the 58 bytes in between are skipped.
#[repr(C)]
struct ImageDosHeader {
    e_magic: u16,         // offset 0 — must equal 0x5A4D ('MZ')
    _reserved: [u8; 58], // offsets 2–59 — not accessed
    e_lfanew: i32,        // offset 60 — byte offset to IMAGE_NT_HEADERS64
}

/// IMAGE_FILE_HEADER (20 bytes).
#[repr(C)]
struct ImageFileHeader {
    machine: u16,
    number_of_sections: u16,
    time_date_stamp: u32,
    pointer_to_symbol_table: u32,
    number_of_symbols: u32,
    size_of_optional_header: u16,
    characteristics: u16,
}

/// One entry in the DataDirectory array.
#[repr(C)]
struct ImageDataDirectory {
    virtual_address: u32,
    size: u32,
}

/// IMAGE_OPTIONAL_HEADER64 (240 bytes, PE32+ / x86-64).
/// Only `data_directory` is accessed at runtime; all fields are present for
/// layout correctness.
#[repr(C)]
struct ImageOptionalHeader64 {
    magic: u16,
    major_linker_version: u8,
    minor_linker_version: u8,
    size_of_code: u32,
    size_of_initialized_data: u32,
    size_of_uninitialized_data: u32,
    address_of_entry_point: u32,
    base_of_code: u32,
    image_base: u64,
    section_alignment: u32,
    file_alignment: u32,
    major_os_version: u16,
    minor_os_version: u16,
    major_image_version: u16,
    minor_image_version: u16,
    major_subsystem_version: u16,
    minor_subsystem_version: u16,
    win32_version_value: u32,
    size_of_image: u32,
    size_of_headers: u32,
    check_sum: u32,
    subsystem: u16,
    dll_characteristics: u16,
    size_of_stack_reserve: u64,
    size_of_stack_commit: u64,
    size_of_heap_reserve: u64,
    size_of_heap_commit: u64,
    loader_flags: u32,
    number_of_rva_and_sizes: u32,
    data_directory: [ImageDataDirectory; 16],
}

/// IMAGE_NT_HEADERS64 (264 bytes).
#[repr(C)]
struct ImageNtHeaders64 {
    signature: u32,
    file_header: ImageFileHeader,
    optional_header: ImageOptionalHeader64,
}

/// IMAGE_IMPORT_DESCRIPTOR (20 bytes).
#[repr(C)]
struct ImageImportDescriptor {
    original_first_thunk: u32,
    time_date_stamp: u32,
    forwarder_chain: u32,
    name: u32,
    first_thunk: u32,
}

/// IMAGE_THUNK_DATA64 (8 bytes).
#[repr(C)]
struct ImageThunkData64 {
    address_or_rva: u64,
}

/// IMAGE_IMPORT_BY_NAME.
#[repr(C)]
struct ImageImportByName {
    hint: u16,
    name: [u8; 1],
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Walk the IAT of the current process's main module, find the IAT slot for
/// `target_func` imported from `target_dll`, overwrite it with `hook_fn`, and
/// return the original function address.
///
/// Call again with the original address as `hook_fn` to restore.
///
/// `target_dll`  — lower-case ASCII bytes without null, e.g. `b"user32.dll"`
/// `target_func` — exact export name bytes without null, e.g. `b"GetSysColor"`
/// `hook_fn`     — address of the replacement function
///
/// # Errors
/// Returns a human-readable string on the first failure.
///
/// # Safety
/// Reads raw memory from the process image. Must be called from DllMain.
pub unsafe fn find_and_patch_iat(
    target_dll: &[u8],
    target_func: &[u8],
    hook_fn: usize,
) -> Result<usize, String> {
    let base = GetModuleHandleW(None)
        .map_err(|e| format!("GetModuleHandleW failed: {e}"))?
        .0 as *const u8;

    let dos = &*(base as *const ImageDosHeader);
    if dos.e_magic != 0x5A4D {
        return Err("Invalid DOS magic (expected MZ / 0x5A4D)".into());
    }

    let nt = &*(base.add(dos.e_lfanew as usize) as *const ImageNtHeaders64);
    if nt.signature != 0x0000_4550 {
        return Err("Invalid PE signature (expected PE\\0\\0 / 0x00004550)".into());
    }

    let import_dir = &nt.optional_header.data_directory[1];
    if import_dir.virtual_address == 0 {
        return Err("Module has no import directory".into());
    }

    let mut desc = base.add(import_dir.virtual_address as usize)
        as *const ImageImportDescriptor;

    while (*desc).name != 0 {
        let dll_ptr = base.add((*desc).name as usize) as *const i8;
        let dll_bytes = CStr::from_ptr(dll_ptr).to_bytes();
        let dll_match = dll_bytes.len() == target_dll.len()
            && dll_bytes
                .iter()
                .zip(target_dll.iter())
                .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase());

        if dll_match {
            let ilt = base.add((*desc).original_first_thunk as usize)
                as *const ImageThunkData64;
            let iat = base.add((*desc).first_thunk as usize)
                as *mut ImageThunkData64;

            let mut i = 0usize;
            while (*ilt.add(i)).address_or_rva != 0 {
                let ilt_val = (*ilt.add(i)).address_or_rva;

                if ilt_val & (1u64 << 63) == 0 {
                    let ibn = base.add(ilt_val as usize) as *const ImageImportByName;
                    let func_bytes =
                        CStr::from_ptr((*ibn).name.as_ptr() as *const i8).to_bytes();

                    if func_bytes == target_func {
                        let iat_slot = &mut (*iat.add(i)).address_or_rva as *mut u64
                            as *mut usize;

                        let mut old_prot = PAGE_PROTECTION_FLAGS(0);
                        VirtualProtect(
                            iat_slot as *mut _,
                            size_of::<usize>(),
                            PAGE_READWRITE,
                            &mut old_prot,
                        )
                        .map_err(|e| format!("VirtualProtect(rw) failed: {e}"))?;

                        let original = *iat_slot;
                        *iat_slot = hook_fn;

                        VirtualProtect(
                            iat_slot as *mut _,
                            size_of::<usize>(),
                            old_prot,
                            &mut old_prot,
                        )
                        .ok();

                        return Ok(original);
                    }
                }
                i += 1;
            }
            return Err(format!(
                "'{}' not found in '{}' import thunks",
                core::str::from_utf8(target_func).unwrap_or("?"),
                core::str::from_utf8(target_dll).unwrap_or("?"),
            ));
        }

        desc = desc.add(1);
    }

    Err(format!(
        "DLL '{}' not found in main module import table",
        core::str::from_utf8(target_dll).unwrap_or("?"),
    ))
}
