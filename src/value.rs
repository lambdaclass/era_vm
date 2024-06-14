use u256::U256;

/// In the zkEVM, all data in the stack and on registers is tagged to determine
/// whether they are a pointer or not.
#[derive(Debug, Clone, Copy)]
pub struct TaggedValue {
    pub value: U256,
    pub is_pointer: bool,
}

pub struct FatPointer {
    offset: u32,
    page: u32,
    start: u32,
    len: u32
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
        let mut encoded: [u8;32] = [0;32];
        for i in 16..20 {
            encoded[i] = (self.offset >> (i * 8)) as u8;
        }
        for i in 20..24 {
            encoded[i] = (self.page >> (i * 8)) as u8;
        }
        for i in 24..28 {
            encoded[i] = (self.start >> (i * 8)) as u8;
        }
        for i in 28..32 {
            encoded[i] = (self.len >> (i * 8)) as u8;
        }
        U256::from(encoded)
    }

    pub fn decode(encoded: U256) -> Self {
        let offset = encoded.byte(16) as u32;
        let page = encoded.byte(20) as u32;
        let start = encoded.byte(24) as u32;
        let len = encoded.byte(28) as u32;
        Self { offset, page, start, len }
    }
}
