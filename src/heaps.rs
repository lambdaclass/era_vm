use zkevm_opcode_defs::system_params::NEW_FRAME_MEMORY_STIPEND;

use crate::state::Heap;

#[derive(Debug, Clone)]
pub struct Heaps {
    pub heaps: Vec<Heap>,
}

impl Default for Heaps {
    fn default() -> Self {
        Self { heaps: vec![] }
    }
}

impl Heaps {
    pub fn new(calldata: Vec<u8>) -> Self {
        // The first heap can never be used because heap zero
        // means the current heap in precompile calls
        let mut heaps = Vec::new();
        heaps.push(Heap::default());
        heaps.push(Heap::new(calldata));
        heaps.push(Heap::default());
        heaps.push(Heap::default());

        Self { heaps }
    }

    pub fn allocate(&mut self) -> u32 {
        let id = self.heaps.len() as u32;
        self.heaps
            .push(Heap::new(vec![0; NEW_FRAME_MEMORY_STIPEND as usize]));
        id
    }

    pub fn allocate_copy(&mut self) -> u32 {
        let id = self.heaps.len() as u32;
        self.heaps
            .push(Heap::new(vec![0; NEW_FRAME_MEMORY_STIPEND as usize]));
        id
    }

    pub fn deallocate(&mut self, heap: u32) {
        self.heaps[heap as usize] = Heap::default();
    }

    pub fn get(&self, index: u32) -> Option<&Heap> {
        self.heaps.get(index as usize)
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut Heap> {
        self.heaps.get_mut(index as usize)
    }
}
