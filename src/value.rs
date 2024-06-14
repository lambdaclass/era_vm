use u256::U256;

/// In the zkEVM, all data in the stack and on registers is tagged to determine
/// whether they are a pointer or not.
#[derive(Debug, Clone, Copy)]
pub struct TaggedValue {
    pub value: U256,
    pub is_pointer: bool,
}
#[derive(Debug)]
pub struct FatPointer {
    pub offset: u32,
    pub page: u32,
    pub start: u32,
    pub len: u32
}

impl Default for FatPointer {
    fn default() -> Self {
        Self {
            offset: 0,
            page: 0,
            start: 0,
            len: 0
        }
    }
}

impl Default for TaggedValue {
    fn default() -> Self {
        Self {
            value: U256::zero(),
            is_pointer: false,
        }
    }
}

impl TaggedValue {
    pub fn new(value: U256, is_pointer: bool) -> Self {
        Self { value, is_pointer }
    }
    pub fn new_raw_integer(value: U256) -> Self {
        Self {
            value,
            is_pointer: false,
        }
    }
    pub fn new_pointer(value: U256) -> Self {
        Self {
            value,
            is_pointer: true,
        }
    }
}

impl std::ops::Add<TaggedValue> for TaggedValue {
    type Output = TaggedValue;

    fn add(self, _rhs: TaggedValue) -> TaggedValue {
        TaggedValue{
            value: self.value + _rhs.value,
            is_pointer: false
        }
    }
}


impl FatPointer {
    pub fn encode(&self) -> U256 {
        let lower_128: u128 = ((self.offset as u128) << 96) | ((self.page as u128) << 64) | ((self.start as u128) << 32) | (self.len as u128);

        U256::from(lower_128)
    }

    pub fn decode(encoded: U256) -> Self {
        let lower_128: u128 = encoded.low_u128();

        // Extract each u32 value from the u128
        let offset: u32 = ((lower_128 >> 96) & 0xFFFFFFFF) as u32;
        let page: u32 = ((lower_128 >> 64) & 0xFFFFFFFF) as u32;
        let start: u32 = ((lower_128 >> 32) & 0xFFFFFFFF) as u32;
        let len: u32 = (lower_128 & 0xFFFFFFFF) as u32;

        Self { offset, page, start, len }
    }
}
