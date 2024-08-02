use u256::{H160, U256};

pub fn address_into_u256(address: H160) -> U256 {
    let mut buffer = [0; 32];
    buffer[12..].copy_from_slice(address.as_bytes());
    U256::from_big_endian(&buffer)
}

pub(crate) fn is_kernel(address: &H160) -> bool {
    address.0[..18].iter().all(|&byte| byte == 0)
}
