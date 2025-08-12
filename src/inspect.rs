//! Reads the memory of the Solitaire process to extract the game state.

use crate::board::{Board, Card, TOTAL_FOUNDATIONS, TOTAL_TABLEAUS, Tableau};

use anyhow::{Context, Result, anyhow, bail};
use std::ffi::{OsStr, OsString};
use std::ops::Drop;
use std::os::windows::ffi::OsStringExt;

use sysinfo::System;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, MODULEENTRY32W, Module32FirstW, TH32CS_SNAPMODULE,
    TH32CS_SNAPMODULE32,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};

const PROCESS_NAME: &str = "Solitaire.exe";
const PILE_LIST_OFFSETS: [usize; 3] = [0xBAFA8, 0x80, 0x98];
const DRAW_OFFSETS: [usize; 3] = [0xBAFA8, 0x48, 0x14];
const STOCK_PILE_INDEX: usize = TOTAL_FOUNDATIONS + TOTAL_TABLEAUS;
const WASTE_PILE_INDEX: usize = STOCK_PILE_INDEX + 1;

// Inspect the current state of the Solitaire game
pub fn inspect() -> Result<Board> {
    let inspector = Inspector::new()?;
    inspector.read()
}

// Check if the Solitaire process is running
pub fn is_running() -> bool {
    get_pid().is_ok()
}

// Get the PID of the Solitaire process
pub fn get_pid() -> Result<u32> {
    let mut system = System::new_all();
    system.refresh_all();
    if let Some(process) = system.processes_by_name(OsStr::new(PROCESS_NAME)).next() {
        Ok(process.pid().as_u32())
    } else {
        bail!("Process '{PROCESS_NAME}' not found. Please ensure {PROCESS_NAME} is running.");
    }
}

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

struct Inspector {
    handle: Handle,
    base_addr: usize,
}

impl Inspector {
    fn new() -> Result<Self> {
        let pid = get_pid()?;
        let handle = Self::get_handle(pid)?;
        let base_addr = Self::get_base_addr(pid)?;
        Ok(Self { handle, base_addr })
    }

    fn read(&self) -> Result<Board> {
        let mut board = Board::new();
        let pile_list = self.read_pile_list().context("Failed to read pile_list")?;
        let draw_count = self
            .read_draw_count()
            .context("Failed to read draw_count")?;
        board.set_draw_count(draw_count as usize);
        for i in 0..TOTAL_FOUNDATIONS {
            let (cards, _) = self.read_pile(&pile_list.piles, TOTAL_FOUNDATIONS - 1 - i)?;
            board.foundations[i] = cards.last().cloned();
        }
        for j in 0..TOTAL_TABLEAUS {
            let (cards, face_up_count) =
                self.read_pile(&pile_list.piles, TOTAL_FOUNDATIONS + TOTAL_TABLEAUS - 1 - j)?;
            board.tableaus[j] = Tableau::new(cards, face_up_count);
        }
        let (stock_cards, _) = self.read_pile(&pile_list.piles, STOCK_PILE_INDEX)?;
        board.stock = stock_cards.into_iter().collect();

        let (waste_cards, _) = self.read_pile(&pile_list.piles, WASTE_PILE_INDEX)?;
        board.waste = waste_cards.into_iter().collect();

        Ok(board)
    }

    fn read_pile(&self, pile_ptrs: &[usize], pile_index: usize) -> Result<(Vec<Card>, usize)> {
        let pile_ptr = pile_ptrs[pile_index];
        let pile_obj =
            self.read_memory::<PileObj>(pile_ptr, &format!("pile_list.piles[{pile_index}]"))?;
        let mut cards = vec![];
        let mut uncovered_count = 0;
        if pile_obj.card_count > 0 {
            let card_list_obj = self.read_memory::<CardListObj>(
                pile_obj.card_list,
                &format!("pile_list.piles[{pile_index}].card_list"),
            )?;
            for j in 0..pile_obj.card_count {
                let card_ptr = card_list_obj.cards[j as usize];
                if card_ptr == 0 {
                    continue;
                }
                let card_obj = self.read_memory::<CardObj>(
                    card_ptr,
                    &format!("pile_list.piles[{pile_index}].card_list.cards[{j}]"),
                )?;
                if card_obj.uncovered == 1 {
                    uncovered_count += 1;
                }
                let card_name = self.read_wstr(
                    card_obj.name,
                    &format!("pile_list.piles[{pile_index}].card_list.cards[{j}].name"),
                )?;
                let card: Card = parse_card(&card_name)
                    .ok_or_else(|| anyhow!("Failed to parse card from '{card_name}'"))?;
                cards.push(card);
            }
        }
        if pile_index == WASTE_PILE_INDEX {
            uncovered_count = pile_obj.waste_uncovered_count as usize;
        }
        Ok((cards, uncovered_count))
    }

    fn read_pointer_chain(&self, offsets: &[usize]) -> Result<usize> {
        let mut ptr = self.base_addr;
        let mut ptr_note = String::from("<base_addr>");
        for offset in offsets.iter() {
            ptr_note = format!("[{ptr_note}+{offset:#x}]");
            ptr = self.read_memory::<usize>(ptr + offset, &ptr_note)?;
        }
        Ok(ptr)
    }

    fn read_pile_list(&self) -> Result<PileListObj> {
        let ptr = self.read_pointer_chain(&PILE_LIST_OFFSETS)?;
        self.read_memory::<PileListObj>(ptr, "<pile_list_ptr>")
    }

    fn read_draw_count(&self) -> Result<u8> {
        let value = self.read_pointer_chain(&DRAW_OFFSETS)?;
        Ok(value as u8)
    }

    fn read_memory<T>(&self, ptr: usize, ptr_note: &str) -> Result<T> {
        let mut buffer: T = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            ReadProcessMemory(
                self.handle.0,
                ptr as _,
                &mut buffer as *mut _ as _,
                std::mem::size_of::<T>(),
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            bail!("Failed to read memory at {ptr:#x} ({ptr_note})");
        } else {
            Ok(buffer)
        }
    }

    fn read_wstr(&self, ptr: usize, ptr_note: &str) -> Result<String> {
        let mut buffer = [0u16; 64];
        let ok = unsafe {
            ReadProcessMemory(
                self.handle.0,
                ptr as _,
                buffer.as_mut_ptr() as _,
                buffer.len() * 2,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            bail!("Failed to read string at {ptr:#x} ({ptr_note})");
        } else {
            let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            Ok(OsString::from_wide(&buffer[..len])
                .to_string_lossy()
                .into_owned())
        }
    }

    fn get_base_addr(pid: u32) -> Result<usize> {
        let snapshot = Handle(unsafe {
            CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid)
        });
        if snapshot.0.is_null() {
            bail!("Failed to create module snapshot");
        }

        let mut module_entry: MODULEENTRY32W = unsafe { std::mem::zeroed() };
        module_entry.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;

        if unsafe { Module32FirstW(snapshot.0, &mut module_entry) } == 0 {
            bail!("Failed to get module");
        }

        Ok(module_entry.modBaseAddr as usize)
    }

    fn get_handle(pid: u32) -> Result<Handle> {
        let handle =
            Handle(unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) });
        if handle.0.is_null() {
            bail!("Failed to open process with PID {pid}");
        }
        Ok(handle)
    }
}

fn parse_card(card_str: &str) -> Option<Card> {
    let mut parts = card_str.split("Of");
    let rank_str = parts.next()?;
    let rank = match rank_str.trim() {
        "Ace" => 0,
        "Two" => 1,
        "Three" => 2,
        "Four" => 3,
        "Five" => 4,
        "Six" => 5,
        "Seven" => 6,
        "Eight" => 7,
        "Nine" => 8,
        "Ten" => 9,
        "Jack" => 10,
        "Queen" => 11,
        "King" => 12,
        _ => return None,
    };
    let suit_str = parts.next()?;
    let suit = match suit_str.trim() {
        "Diamonds" => 0,
        "Clubs" => 1,
        "Hearts" => 2,
        "Spades" => 3,
        _ => return None,
    };
    Some(Card::new_with_rank_suit(rank, suit))
}

#[repr(C)]
struct CardObj {
    _unused1: [u8; 0x11],
    uncovered: u8,
    _unused2: [u8; 0x26],
    name: usize,
}

#[repr(C)]
struct CardListObj {
    cards: [usize; 52],
}

#[repr(C)]
struct PileObj {
    _unused1: [u8; 0x30],
    waste_uncovered_count: i32,
    _unused2: [u8; 0xFC],
    card_count: i32,
    _unused3: [u8; 12],
    card_list: usize,
}

#[repr(C)]
struct PileListObj {
    piles: [usize; 13],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect() {
        match is_running() {
            true => {
                let board = inspect().unwrap();
                assert!(
                    board.draw_count() == 1 || board.draw_count() == 3,
                    "Draw count should be 1 or 3"
                );
                println!("{}", board.to_pretty_string());
            }
            false => {
                eprintln!("Solitaire is not running.");
            }
        }
    }
}
