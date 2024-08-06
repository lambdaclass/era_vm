use u256::{H160, U256};

pub fn address_into_u256(address: H160) -> U256 {
    let mut buffer = [0; 32];
    buffer[12..].copy_from_slice(address.as_bytes());
    U256::from_big_endian(&buffer)
}

pub(crate) fn is_kernel(address: &H160) -> bool {
    address.0[..18].iter().all(|&byte| byte == 0)
}

pub trait LowUnsigned {
    fn low_u64(&self) -> u64;

    fn low_u32(&self) -> u32 {
        (self.low_u64() & u32::MAX as u64) as u32
    }

    fn low_u16(&self) -> u16 {
        (self.low_u64() & u16::MAX as u64) as u16
    }
}

impl LowUnsigned for U256 {
    fn low_u64(&self) -> u64 {
        self.low_u64()
    }
}

impl LowUnsigned for u64 {
    fn low_u64(&self) -> u64 {
        *self
    }
}

impl LowUnsigned for u32 {
    fn low_u64(&self) -> u64 {
        *self as u64
    }
}
