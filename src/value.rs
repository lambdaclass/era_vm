use u256::U256;

/// In the zkEVM, all data in the stack and on registers is tagged to determine
/// whether they are a pointer or not.
#[derive(Debug, Clone, Copy)]
pub struct TaggedValue {
    pub value: U256,
    pub is_pointer: bool,
}
#[derive(Debug, Default)]
pub struct FatPointer {
    pub offset: u32,
    pub page: u32,
    pub start: u32,
    pub len: u32,
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

    pub fn to_raw_integer(&mut self) {
        self.is_pointer = false;
    }

    pub fn zero() -> Self {
        Self {
            value: U256::zero(),
            is_pointer: false,
        }
    }
}

impl std::ops::Add<TaggedValue> for TaggedValue {
    type Output = TaggedValue;

    fn add(self, _rhs: TaggedValue) -> TaggedValue {
        TaggedValue {
            value: self.value + _rhs.value,
            is_pointer: false,
        }
    }
}

impl std::ops::BitOrAssign<TaggedValue> for TaggedValue {
    fn bitor_assign(&mut self, rhs: TaggedValue) {
        self.value |= rhs.value;
    }
}

impl FatPointer {
    pub fn encode(&self) -> U256 {
        let mut result = U256::zero();
        result.0[0] = (self.offset as u64) | ((self.page as u64) << 32);

        result.0[1] = (self.start as u64) | ((self.len as u64) << 32);

        result
    }

    pub fn decode(value: U256) -> Self {
        let raw_value = value.0;
        let offset = raw_value[0] as u32;
        let page = (raw_value[0] >> 32) as u32;

        let start = raw_value[1] as u32;
        let len = (raw_value[1] >> 32) as u32;

        Self {
            offset,
            page,
            start,
            len,
        }
    }

    pub fn narrow(&mut self) {
        self.start += self.offset;
        self.len -= self.offset;
        self.offset = 0;
    }
}
