use zkevm_opcode_defs::system_params::NEW_FRAME_MEMORY_STIPEND;

use crate::{eravm_error::HeapError, execution::Heap};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Heaps {
    heaps: Vec<Heap>,
}

impl Heaps {
    pub fn new(calldata: Vec<u8>) -> Self {
        // The first heap can never be used because heap zero
        // means the current heap in precompile calls
        let size = calldata.len() as u32;
        let heaps = vec![
            Heap::default(),
            Heap::new(calldata, size),
            Heap::default(),
            Heap::default(),
        ];

        Self { heaps }
    }

    pub fn allocate(&mut self) -> u32 {
        let id = self.heaps.len() as u32;
        self.heaps.push(Heap::new(vec![], NEW_FRAME_MEMORY_STIPEND));
        id
    }

    pub fn allocate_copy(&mut self) -> u32 {
        let id: u32 = self.heaps.len() as u32;
        self.heaps.push(Heap::new(vec![], NEW_FRAME_MEMORY_STIPEND));
        id
    }

    pub fn deallocate(&mut self, heap: u32) {
        self.heaps[heap as usize] = Heap::default();
    }

    pub fn get(&self, index: u32) -> Option<&Heap> {
        self.heaps.get(index as usize)
    }

    pub fn try_get(&self, index: u32) -> Result<&Heap, HeapError> {
        self.get(index).ok_or(HeapError::ReadOutOfBounds)
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut Heap> {
        self.heaps.get_mut(index as usize)
    }

    pub fn try_get_mut(&mut self, index: u32) -> Result<&mut Heap, HeapError> {
        self.get_mut(index).ok_or(HeapError::ReadOutOfBounds)
    }
}
