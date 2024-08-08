use zkevm_opcode_defs::{ethereum_types::U256, p256};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Secp256r1VerifyPrecompile;

impl Precompile for Secp256r1VerifyPrecompile {
    fn execute_precompile(&mut self, query: U256, heaps: &mut Heaps) {
        let precompile_call_params = query;
        let params = precompile_abi_in_log(precompile_call_params);

        let mut current_read_location = MemoryLocation {
            page: params.memory_page_to_read,
            index: params.input_memory_offset,
        };

        let hash_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let hash_query = heaps.new_execute_partial_query(hash_query);
        let hash_value = hash_query.value;

        current_read_location.index += 1;
        let r_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let r_query = heaps.new_execute_partial_query(r_query);
        let r_value = r_query.value;

        current_read_location.index += 1;
        let s_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let s_query = heaps.new_execute_partial_query(s_query);
        let s_value = s_query.value;

        current_read_location.index += 1;
        let x_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let x_query = heaps.new_execute_partial_query(x_query);
        let x_value = x_query.value;

        current_read_location.index += 1;
        let y_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let y_query = heaps.new_execute_partial_query(y_query);
        let y_value = y_query.value;

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

        if let Ok(is_valid) = result {
            let mut write_location = MemoryLocation {
                page: params.memory_page_to_write,
                index: params.output_memory_offset,
            };

            let ok_marker = U256::one();
            let ok_or_err_query = MemoryQuery {
                location: write_location,
                value: ok_marker,
                value_is_pointer: false,
                rw_flag: true,
            };
            heaps.new_execute_partial_query(ok_or_err_query);

            write_location.index += 1;
            let result = U256::from(is_valid as u64);
            let result_query = MemoryQuery {
                location: write_location,
                value: result,
                value_is_pointer: false,
                rw_flag: true,
            };
            heaps.new_execute_partial_query(result_query);
        } else {
            let mut write_location = MemoryLocation {
                page: params.memory_page_to_write,
                index: params.output_memory_offset,
            };

            let err_marker = U256::zero();
            let ok_or_err_query = MemoryQuery {
                location: write_location,
                value: err_marker,
                value_is_pointer: false,
                rw_flag: true,
            };
            heaps.new_execute_partial_query(ok_or_err_query);

            write_location.index += 1;
            let empty_result = U256::zero();
            let result_query = MemoryQuery {
                location: write_location,
                value: empty_result,
                value_is_pointer: false,
                rw_flag: true,
            };
            heaps.new_execute_partial_query(result_query);
        }
    }
}

pub fn secp256r1_verify_inner(
    digest: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
    x: &[u8; 32],
    y: &[u8; 32],
) -> Result<bool, ()> {
    use p256::ecdsa::signature::hazmat::PrehashVerifier;
    use p256::ecdsa::{Signature, VerifyingKey};
    use p256::elliptic_curve::generic_array::GenericArray;
    use p256::elliptic_curve::sec1::FromEncodedPoint;
    use p256::{AffinePoint, EncodedPoint};

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

pub fn secp256r1_verify(precompile_call_params: U256, heaps: &mut Heaps) {
    let mut processor = Secp256r1VerifyPrecompile;
    processor.execute_precompile(precompile_call_params, heaps);
}
