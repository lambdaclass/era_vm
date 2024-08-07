use u256::U256;
use zkevm_opcode_defs::system_params::NEW_FRAME_MEMORY_STIPEND;

use crate::state::Heap;

#[derive(Debug, Clone, Default)]
pub struct Heaps {
    heaps: Vec<Heap>,
}

impl Heaps {
    pub fn new(calldata: Vec<u8>) -> Self {
        // The first heap can never be used because heap zero
        // means the current heap in precompile calls
        let heaps = vec![
            Heap::default(),
            Heap::new(calldata),
            Heap::default(),
            Heap::default(),
        ];

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

    // TODO: Move somewhere else?
    pub fn new_execute_partial_query(&mut self, mut query: MemoryQuery) -> MemoryQuery {
        let page = query.location.page;

        let start = query.location.index * 32;
        if query.rw_flag {
            self.get_mut(page).unwrap().store(start, query.value);
        } else {
            self.get_mut(page).unwrap().expand_memory(start + 32);
            query.value = self.get(page).unwrap().read(start);
            query.value_is_pointer = false;
        }
        query
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemoryQuery {
    pub location: MemoryLocation,
    pub value: U256,
    pub rw_flag: bool,
    pub value_is_pointer: bool,
}

impl MemoryQuery {
    pub fn empty() -> Self {
        Self {
            location: MemoryLocation { page: 0, index: 0 },
            value: U256::zero(),
            rw_flag: false,
            value_is_pointer: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    pub page: u32,
    pub index: u32,
}

pub struct PrecompileCallABI {
    pub input_memory_offset: u32,
    pub input_memory_length: u32,
    pub output_memory_offset: u32,
    pub output_memory_length: u32,
    pub memory_page_to_read: u32,
    pub memory_page_to_write: u32,
    pub precompile_interpreted_data: u64,
}

impl PrecompileCallABI {
    pub const fn from_u256(raw_value: U256) -> Self {
        let raw = raw_value.0;
        let input_memory_offset = raw[0] as u32;
        let input_memory_length = (raw[0] >> 32) as u32;
        let output_memory_offset = raw[1] as u32;
        let output_memory_length = (raw[1] >> 32) as u32;
        let memory_page_to_read = raw[2] as u32;
        let memory_page_to_write = (raw[2] >> 32) as u32;
        let precompile_interpreted_data = raw[3];

        Self {
            input_memory_offset,
            input_memory_length,
            output_memory_offset,
            output_memory_length,
            memory_page_to_read,
            memory_page_to_write,
            precompile_interpreted_data,
        }
    }
}

pub fn precompile_abi_in_log(abi_key: U256) -> PrecompileCallABI {
    PrecompileCallABI::from_u256(abi_key)
}
