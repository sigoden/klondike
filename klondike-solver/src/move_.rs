use super::*;

use std::cmp::Ordering;

#[derive(Debug, Copy, Clone, Default)]
pub struct Move {
    pub value1: u8,
    pub value2: u8,
}

impl Move {
    pub fn new(from: u8, to: u8, count: u8, flip: bool) -> Self {
        Move {
            value1: from | (to << 4),
            value2: count | if flip { 0x80 } else { 0x00 },
        }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.value1 == 0
    }

    #[inline]
    pub fn from(&self) -> u8 {
        self.value1 & 0x0f
    }

    #[inline]
    pub fn to(&self) -> u8 {
        self.value1 >> 4
    }

    #[inline]
    pub fn count(&self) -> usize {
        (self.value2 & 0x7f) as usize
    }

    #[inline]
    pub fn flip(&self) -> bool {
        (self.value2 & 0x80) != 0
    }

    #[inline]
    pub fn values(&self) -> (usize, usize, usize, bool) {
        (
            self.from() as usize,
            self.to() as usize,
            self.count(),
            self.flip(),
        )
    }
}

impl PartialEq for Move {
    fn eq(&self, other: &Self) -> bool {
        self.value1 == other.value1
    }
}

impl Eq for Move {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MoveIndex {
    pub index: u32,
    pub priority: i16,
    pub estimate: Estimate,
}

impl MoveIndex {
    pub fn new(index: u32, priority: i16, estimate: Estimate) -> Self {
        MoveIndex {
            index,
            priority,
            estimate,
        }
    }
}

impl Ord for MoveIndex {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for MoveIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Default)]
pub struct MoveNode {
    pub parent: u32,
    pub mov: Move,
}

impl MoveNode {
    pub fn copy(&self, destination: &mut [Move], nodes: &[MoveNode]) -> usize {
        let mut index = 0;
        if self.mov.is_null() {
            return 0;
        }

        destination[index] = self.mov;
        index += 1;
        let mut current_parent = self.parent;
        while current_parent > 0 {
            let parent = nodes[current_parent as usize];
            if parent.mov.is_null() {
                break;
            }

            destination[index] = parent.mov;
            index += 1;
            current_parent = parent.parent;
        }
        index
    }
}
