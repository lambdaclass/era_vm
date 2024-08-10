use super::{precompile_abi_in_log, Precompile};
use crate::{eravm_error::EraVmError, heaps::Heaps};
use p256::{
    ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey},
    elliptic_curve::{generic_array::GenericArray, sec1::FromEncodedPoint},
    AffinePoint, EncodedPoint,
};
use u256::U256;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Secp256r1VerifyPrecompile;

impl Precompile for Secp256r1VerifyPrecompile {
    fn execute_precompile(&mut self, query: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
        let precompile_call_params = query;
        let params = precompile_abi_in_log(precompile_call_params);
        let addr = |offset: u32| (params.output_memory_offset + offset) * 32;

        let read_heap = heaps.try_get_mut(params.memory_page_to_read)?;
        let (hash_value, _) = read_heap.expanded_read(addr(0));
        let (r_value, _) = read_heap.expanded_read(addr(1));
        let (s_value, _) = read_heap.expanded_read(addr(2));
        let (x_value, _) = read_heap.expanded_read(addr(3));
        let (y_value, _) = read_heap.expanded_read(addr(4));

        // read everything as bytes for ecrecover purposes

        let mut buffer = [0u8; 32];
        hash_value.to_big_endian(&mut buffer[..]);
        let hash = buffer;

        r_value.to_big_endian(&mut buffer[..]);
        let r_bytes = buffer;

        s_value.to_big_endian(&mut buffer[..]);
        let s_bytes = buffer;

        x_value.to_big_endian(&mut buffer[..]);
        let x_bytes = buffer;

        y_value.to_big_endian(&mut buffer[..]);
        let y_bytes = buffer;

        let result = secp256r1_verify_inner(&hash, &r_bytes, &s_bytes, &x_bytes, &y_bytes);

        let (marker, result) = match result {
            Ok(is_valid) => (U256::one(), U256::from(is_valid as u64)),
            _ => (U256::zero(), U256::zero()),
        };

        let write_heap = heaps.try_get_mut(params.memory_page_to_write)?;
        write_heap.store(addr(0), marker);
        write_heap.store(addr(1), result);

        Ok(())
    }
}

pub fn secp256r1_verify_inner(
    digest: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
    x: &[u8; 32],
    y: &[u8; 32],
) -> Result<bool, ()> {
    // we expect pre-validation, so this check always works
    let signature = Signature::from_scalars(
        GenericArray::clone_from_slice(r),
        GenericArray::clone_from_slice(s),
    )
    .map_err(|_| ())?;

    let encoded_pk = EncodedPoint::from_affine_coordinates(
        &GenericArray::clone_from_slice(x),
        &GenericArray::clone_from_slice(y),
        false,
    );

    let may_be_pk_point = AffinePoint::from_encoded_point(&encoded_pk);
    if bool::from(may_be_pk_point.is_none()) {
        return Err(());
    }
    let pk_point = may_be_pk_point.unwrap();

    let verifier = VerifyingKey::from_affine(pk_point).map_err(|_| ())?;

    let result = verifier.verify_prehash(digest, &signature);

    Ok(result.is_ok())
}

// Verifies an ECDSA signature against a message digest using a given public key.
pub fn secp256r1_verify_function(abi_key: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
    Secp256r1VerifyPrecompile.execute_precompile(abi_key, heaps)
}
