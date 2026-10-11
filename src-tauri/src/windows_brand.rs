use tauri::WebviewWindow;
use windows::core::PWSTR;
use windows::Win32::{
    Foundation::HWND,
    Storage::EnhancedStorage::{PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchIconResource},
    System::{
        Com::{
            CoTaskMemAlloc,
            StructuredStorage::{PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0},
        },
        Variant::VT_LPWSTR,
    },
    UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
};
use windows_sys::Win32::{
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{LoadIconW, SendMessageW, ICON_BIG, WM_SETICON},
};

pub const TASKBAR_APP_ID: &str = "com.ultrastt.voicekeyboard";

pub fn set_taskbar_icon(window: &WebviewWindow, app_id: &str) -> tauri::Result<()> {
    let handle = window.hwnd()?.0;
    let icon = unsafe { LoadIconW(GetModuleHandleW(std::ptr::null()), 32512usize as *const u16) };
    if icon.is_null() {
        return Err(tauri::Error::Io(std::io::Error::last_os_error()));
    }
    unsafe {
        SendMessageW(handle, WM_SETICON, ICON_BIG as usize, icon as isize);
    }

    let hwnd = HWND(handle);
    let property_store = unsafe { SHGetPropertyStoreForWindow::<IPropertyStore>(hwnd) }
        .map_err(|error| tauri::Error::Io(std::io::Error::other(error.to_string())))?;
    let icon_resource = format!("{},0", std::env::current_exe()?.display());
    set_string_property(
        &property_store,
        &PKEY_AppUserModel_RelaunchIconResource,
        &icon_resource,
    )
    .map_err(|error| tauri::Error::Io(std::io::Error::other(error.to_string())))?;
    set_string_property(&property_store, &PKEY_AppUserModel_ID, app_id)
        .map_err(|error| tauri::Error::Io(std::io::Error::other(error.to_string())))?;
    unsafe { property_store.Commit() }
        .map_err(|error| tauri::Error::Io(std::io::Error::other(error.to_string())))?;
    Ok(())
}

fn set_string_property(
    store: &IPropertyStore,
    key: &windows::Win32::Foundation::PROPERTYKEY,
    value: &str,
) -> windows::core::Result<()> {
    let encoded = value
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let allocation_size = encoded.len() * std::mem::size_of::<u16>();
    let buffer = unsafe { CoTaskMemAlloc(allocation_size) }.cast::<u16>();
    if buffer.is_null() {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(0x8007000E_u32 as i32),
            "Could not allocate a Windows property string",
        ));
    }
    unsafe {
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), buffer, encoded.len());
    }
    let variant = PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_LPWSTR,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 {
                    pwszVal: PWSTR(buffer),
                },
            }),
        },
    };
    unsafe { store.SetValue(key, &variant) }
}
