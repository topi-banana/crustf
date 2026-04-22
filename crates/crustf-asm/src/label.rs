//! Labels for branch resolution.
//!
//! Labels are opaque identifiers handed out by a [`CodeBuilder`]. Each label
//! can be *placed* at most once (the position of the following instruction
//! in the code array) and *referenced* any number of times by branch
//! instructions. The builder resolves labels to signed 16 or 32 bit byte
//! offsets when the enclosing method is finalised.

use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub(crate) usize);

#[derive(Debug, Default, Clone)]
pub(crate) struct LabelTable {
    /// `positions[label_id] = Some(byte_offset)` once the label is placed.
    positions: Vec<Option<u32>>,
}

impl LabelTable {
    pub fn new_label(&mut self) -> Label {
        let id = self.positions.len();
        self.positions.push(None);
        Label(id)
    }

    pub fn place(&mut self, label: Label, offset: u32) {
        self.positions[label.0] = Some(offset);
    }

    pub fn resolve(&self, label: Label) -> Option<u32> {
        self.positions.get(label.0).copied().flatten()
    }
}
