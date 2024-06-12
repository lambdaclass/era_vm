use u256::U256;

/// In the zkEVM, all data in the stack and on registers is tagged to determine
/// whether they are a pointer or not.
#[derive(Debug, Clone)]
pub struct TaggedValue {
    pub value: U256,
    pub is_pointer: bool,
}
