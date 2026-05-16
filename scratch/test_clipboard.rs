use std::path::Path;
use windows::Win32::Foundation::{HANDLE, CloseHandle};
use windows::Win32::System::DataExchange::{OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GHND};
use windows::Win32::UI::Shell::DROPFILES;

fn main() {
    let path = Path::new(r"C:\Users\ngbal\AppData\Local\MacroNest");
    if !path.exists() {
        println!("Path does not exist!");
        return;
    }

    let path_str = path.to_string_lossy().to_string();
    let mut path_wide: Vec<u16> = path_str.encode_utf16().collect();
    path_wide.push(0);
    path_wide.push(0);

    let size = std::mem::size_of::<DROPFILES>() + (path_wide.len() * 2);
    unsafe {
        let h_global = GlobalAlloc(GHND, size).expect("GlobalAlloc failed");
        let ptr = GlobalLock(h_global);
        if ptr.is_null() {
            println!("GlobalLock failed");
            return;
        }

        std::ptr::write_bytes(ptr, 0, size);

        let drop_files = ptr as *mut DROPFILES;
        (*drop_files).pFiles = std::mem::size_of::<DROPFILES>() as u32;
        (*drop_files).fWide = true.into();

        let dest_path_ptr = (ptr as *mut u8).add(std::mem::size_of::<DROPFILES>()) as *mut u16;
        std::ptr::copy_nonoverlapping(path_wide.as_ptr(), dest_path_ptr, path_wide.len());

        GlobalUnlock(h_global).expect("GlobalUnlock failed");

        if OpenClipboard(None).is_err() {
            println!("OpenClipboard failed");
            return;
        }
        let _ = EmptyClipboard();
        if SetClipboardData(15, Some(HANDLE(h_global.0))).is_err() {
            let _ = CloseClipboard();
            println!("SetClipboardData failed");
            return;
        }
        let _ = CloseClipboard();
    }
    println!("Successfully copied to clipboard! Try pasting now.");
}
