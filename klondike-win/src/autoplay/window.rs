use anyhow::{Result, bail};

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, S_OK};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SW_RESTORE, SetForegroundWindow, ShowWindow,
};

const WINDOW_WIDTH: i32 = 1978;
const WINDOW_HEIGHT: i32 = 1366;
const STOCK_CENTER_X: i32 = 191;
const STOCK_CLICK_Y: i32 = 185; // = Stock Top Y + UNCOVERED_OFFSET_Y
const TABLEAU_TOP_Y: i32 = 464;
const TABLEAU_OFFSET_X: i32 = 266;
const COVERED_OFFSET_Y: i32 = 17;
const UNCOVERED_OFFSET_Y: i32 = 57;
const WASTE_OFFSET_X: i32 = 37;
const COMPACT_TOP_Y: i32 = 1066; // If the top y of the last card exceeds this, compact the uncovered offset.

pub type Point = (i32, i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone)]
pub struct Window {
    rect: Rect,
    factor_x: f32,
    factor_y: f32,
}

impl Window {
    pub fn new(rect: Rect) -> Self {
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let factor_x = width as f32 / WINDOW_WIDTH as f32;
        let factor_y = height as f32 / WINDOW_HEIGHT as f32;
        Window {
            rect,
            factor_x,
            factor_y,
        }
    }

    pub fn stock_point(&self) -> Point {
        self.transform(STOCK_CENTER_X, STOCK_CLICK_Y)
    }

    pub fn waste_point(&self) -> Point {
        self.transform(
            STOCK_CENTER_X + TABLEAU_OFFSET_X + WASTE_OFFSET_X,
            STOCK_CLICK_Y,
        )
    }

    pub fn foundation_point(&self, foundation_index: usize) -> Point {
        self.transform(
            STOCK_CENTER_X + (foundation_index as i32 + 3) * TABLEAU_OFFSET_X,
            STOCK_CLICK_Y,
        )
    }

    pub fn move_to_tableau_point(
        &self,
        tableau_index: usize,
        cards_count: usize,
        uncovered_count: usize,
    ) -> Point {
        self.transform(
            STOCK_CENTER_X + (tableau_index as i32) * TABLEAU_OFFSET_X,
            TABLEAU_TOP_Y
                + (cards_count - uncovered_count) as i32 * COVERED_OFFSET_Y
                + uncovered_count as i32 * UNCOVERED_OFFSET_Y
                + UNCOVERED_OFFSET_Y / 2,
        )
    }

    pub fn move_from_tableau_point(
        &self,
        tableau_index: usize,
        cards_count: usize,
        uncovered_count: usize,
        moved_count: usize,
    ) -> Point {
        let get_top_y = |uncovered_offset_y: i32| {
            TABLEAU_TOP_Y
                + (cards_count - uncovered_count) as i32 * COVERED_OFFSET_Y
                + (uncovered_count - 1) as i32 * uncovered_offset_y
        };
        let mut uncovered_offset_y = UNCOVERED_OFFSET_Y;
        let mut top_y = get_top_y(uncovered_offset_y);
        let mut i = 0;
        while top_y > COMPACT_TOP_Y {
            if i < 2 {
                uncovered_offset_y -= 5;
            } else {
                uncovered_offset_y -= 3;
            }
            top_y = get_top_y(uncovered_offset_y);
            i += 1;
        }
        self.transform(
            STOCK_CENTER_X + (tableau_index as i32) * TABLEAU_OFFSET_X,
            TABLEAU_TOP_Y
                + (cards_count - uncovered_count) as i32 * COVERED_OFFSET_Y
                + (uncovered_count - moved_count) as i32 * uncovered_offset_y
                + uncovered_offset_y / 2,
        )
    }

    fn transform(&self, x: i32, y: i32) -> Point {
        (
            (x as f32 * self.factor_x) as i32 + self.rect.left,
            (y as f32 * self.factor_y) as i32 + self.rect.top,
        )
    }
}

/// Get the main window rectangle of the specified PID (left, top, right, bottom)
pub fn get_window_rect(pid: u32) -> Result<(Rect, isize)> {
    struct FindWindowData {
        target_pid: u32,
        found_hwnd: HWND,
    }

    // Store PID in a temporary location for lparam
    let mut data = FindWindowData {
        target_pid: pid,
        found_hwnd: std::ptr::null_mut(),
    };

    unsafe {
        EnumWindows(Some(enum_windows_proc), &mut data as *mut _ as LPARAM);
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        let mut process_id = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut process_id);
        }
        let data = unsafe { &mut *(lparam as *mut FindWindowData) };
        if process_id == data.target_pid && unsafe { IsWindowVisible(hwnd) == 1 } {
            data.found_hwnd = hwnd;
            return 0;
        }
        1
    }

    if data.found_hwnd.is_null() {
        bail!("Main window not found");
    }
    let hwnd = data.found_hwnd;

    unsafe {
        let mut rect = RECT::default();
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as _,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        ) == S_OK
        {
            let rect = Rect {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
            Ok((rect, hwnd as isize))
        } else {
            bail!("Failed to get window rect");
        }
    }
}

pub fn focus_window(hwnd: isize) -> Result<()> {
    let hwnd = hwnd as HWND;
    unsafe {
        if IsIconic(hwnd) == 1 {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        if SetForegroundWindow(hwnd) == 0 {
            bail!("Failed to focus window");
        }
    };
    Ok(())
}

pub fn is_foreground_window(hwnd: isize) -> bool {
    let fg = unsafe { GetForegroundWindow() } as isize;
    fg == hwnd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window() {
        let window_rect = Rect {
            left: 0,
            top: 0,
            right: WINDOW_WIDTH,
            bottom: WINDOW_HEIGHT,
        };
        // let window_rect = solitaire_inspect::get_pid()
        //     .and_then(solitaire_inspect::get_window_rect)
        //     .unwrap();
        let window = Window::new(window_rect);
        assert_eq!(
            (
                window.rect.left,
                window.rect.top,
                window.rect.right - window.rect.left,
                window.rect.bottom - window.rect.top,
            ),
            (0, 0, 1978, 1366),
            "(X, Y, WIDTH, HEIGHT)",
        );
        assert_eq!(window.stock_point(), (191, 185), "Stock point mismatch");
        assert_eq!(
            window.waste_point(),
            (494, 185),
            "Waste (3th) point mismatch"
        );
        assert_eq!(
            window.foundation_point(0),
            (989, 185),
            "Foundation#1 point mismatch"
        );
        assert_eq!(
            window.foundation_point(1),
            (1255, 185),
            "Foundation#2 point mismatch"
        );
        assert_eq!(
            window.foundation_point(3),
            (1787, 185),
            "Foundation#4 point mismatch"
        );

        assert_eq!(
            window.move_to_tableau_point(0, 1, 1),
            (191, 549),
            "To Tableau#1, Count: 1, Uncovered: 1",
        );
        assert_eq!(
            window.move_from_tableau_point(0, 1, 1, 1),
            (191, 492),
            "From Tableau#1, Count: 1, Uncovered: 1, Moved: 1",
        );
        assert_eq!(
            window.move_to_tableau_point(1, 2, 1),
            (457, 566),
            "To Tableau#2, Count: 2, Uncovered: 1",
        );
        assert_eq!(
            window.move_to_tableau_point(6, 7, 1),
            (1787, 651),
            "To Tableau#7, Count: 7,  Uncovered: 1",
        );
        assert_eq!(
            window.move_from_tableau_point(0, 11, 11, 1),
            (191, 1062),
            "From Tableau#1, Cards: K-3, Move: 3",
        );
        assert_eq!(
            window.move_from_tableau_point(0, 12, 12, 1),
            (191, 1062),
            "From Tableau#1, Cards: K-2, Move: 2",
        );
        assert_eq!(
            window.move_from_tableau_point(0, 12, 12, 3),
            (191, 958),
            "From Tableau#1, Cards: K-2, Move: 4",
        );
        assert_eq!(
            window.move_from_tableau_point(6, 12, 6, 1),
            (1787, 879),
            "From Tableau#7, Count: 12, Uncovered: 6, Moved: 1",
        );
    }
}
