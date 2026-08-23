use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

static IS_DOWN: AtomicBool = AtomicBool::new(false);
static mut EVENT_SENDER: Option<SyncSender<bool>> = None;

pub struct HotkeyListener {
    _hook_thread: std::thread::JoinHandle<()>,
    _worker_thread: std::thread::JoinHandle<()>,
}

pub type HotkeyCallback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

impl HotkeyListener {
    pub fn start(callback: HotkeyCallback) -> Self {
        let (tx, rx) = sync_channel::<bool>(64);

        unsafe {
            EVENT_SENDER = Some(tx);
        }

        // Dedicated worker thread that executes the callback WITHOUT blocking the Windows hook
        let worker_thread = std::thread::spawn(move || {
            while let Ok(pressed) = rx.recv() {
                callback(pressed);
            }
        });

        // Dedicated OS thread for the Windows low-level hook
        let hook_thread = std::thread::spawn(|| unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                HINSTANCE::default(),
                0,
            );

            if let Ok(hook_handle) = hook {
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    // Fast message pump
                }
                let _ = UnhookWindowsHookEx(hook_handle);
            }
        });

        Self {
            _hook_thread: hook_thread,
            _worker_thread: worker_thread,
        }
    }
}

unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk_code = kbd.vkCode;
        let is_extended = (kbd.flags.0 & 0x01) != 0;

        // VK_RCONTROL is 0xA3, or VK_CONTROL (0x11) with extended flag set
        let is_right_ctrl = vk_code == 0xA3 || (vk_code == 0x11 && is_extended);

        if is_right_ctrl {
            let msg = wparam.0 as u32;
            let is_press = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let is_release = msg == WM_KEYUP || msg == WM_SYSKEYUP;

            if is_press {
                let was_down = IS_DOWN.swap(true, Ordering::SeqCst);
                if !was_down {
                    if let Some(ref sender) = EVENT_SENDER {
                        let _ = sender.try_send(true);
                    }
                }
            } else if is_release {
                let was_down = IS_DOWN.swap(false, Ordering::SeqCst);
                if was_down {
                    if let Some(ref sender) = EVENT_SENDER {
                        let _ = sender.try_send(false);
                    }
                }
            }
        }
    }

    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}
