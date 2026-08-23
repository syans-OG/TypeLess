use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GlobalSize, GHND};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
};

const CF_UNICODETEXT: u32 = 13;

pub struct TextInjector;

impl TextInjector {
    pub fn paste_text(text: &str) -> anyhow::Result<()> {
        if text.trim().is_empty() {
            return Ok(());
        }

        // 1. Get existing clipboard text to restore later
        let previous_text = Self::get_clipboard_text().unwrap_or_default();

        // 2. Set new text to clipboard
        Self::set_clipboard_text(text)?;

        // 3. Small sleep to let OS register clipboard content
        sleep(Duration::from_millis(30));

        // 4. Send Ctrl + V keystroke
        Self::send_ctrl_v();

        // 5. Sleep slightly to allow the active window to read clipboard
        sleep(Duration::from_millis(150));

        // 6. Restore original clipboard content
        if !previous_text.is_empty() {
            let _ = Self::set_clipboard_text(&previous_text);
        }

        Ok(())
    }

    fn set_clipboard_text(text: &str) -> anyhow::Result<()> {
        let wide: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let bytes_len = wide.len() * std::mem::size_of::<u16>();

        unsafe {
            if OpenClipboard(HWND(null_mut())).is_err() {
                return Err(anyhow::anyhow!("Failed to open clipboard"));
            }

            let _ = EmptyClipboard();

            let h_mem = GlobalAlloc(GHND, bytes_len);
            if h_mem.is_err() {
                let _ = CloseClipboard();
                return Err(anyhow::anyhow!("GlobalAlloc failed"));
            }
            let h_mem = h_mem.unwrap();

            let ptr = GlobalLock(h_mem);
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr as *mut u8, bytes_len);
                let _ = GlobalUnlock(h_mem);
                let _ = SetClipboardData(CF_UNICODETEXT, HANDLE(h_mem.0));
            }

            let _ = CloseClipboard();
        }

        Ok(())
    }

    fn get_clipboard_text() -> Option<String> {
        unsafe {
            if OpenClipboard(HWND(null_mut())).is_err() {
                return None;
            }

            let handle = GetClipboardData(CF_UNICODETEXT);
            if handle.is_err() {
                let _ = CloseClipboard();
                return None;
            }
            let handle = handle.unwrap();

            let size_bytes = GlobalSize(HGLOBAL(handle.0));
            let max_u16 = size_bytes / std::mem::size_of::<u16>();

            let ptr = GlobalLock(HGLOBAL(handle.0));
            let result = if !ptr.is_null() && max_u16 > 0 {
                let wide_slice = std::slice::from_raw_parts(ptr as *const u16, max_u16);
                let len = wide_slice.iter().position(|&c| c == 0).unwrap_or(max_u16);
                let str_res = String::from_utf16_lossy(&wide_slice[..len]);
                let _ = GlobalUnlock(HGLOBAL(handle.0));
                Some(str_res)
            } else {
                if !ptr.is_null() {
                    let _ = GlobalUnlock(HGLOBAL(handle.0));
                }
                None
            };

            let _ = CloseClipboard();
            result
        }
    }

    fn send_ctrl_v() {
        unsafe {
            let inputs = [
                // Ctrl down
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                // V down
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_V,
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                // V up
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_V,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                // Ctrl up
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];

            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}
