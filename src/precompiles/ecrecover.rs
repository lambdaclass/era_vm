use zkevm_opcode_defs::k256::ecdsa::VerifyingKey;
pub use zkevm_opcode_defs::sha2::Digest;
use zkevm_opcode_defs::{ethereum_types::U256, k256, sha3};

use crate::eravm_error::EraVmError;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ECRecoverPrecompile;

impl Precompile for ECRecoverPrecompile {
    fn execute_precompile(&mut self, query: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
        let precompile_call_params = query;
        let params = precompile_abi_in_log(precompile_call_params);

        let mut current_read_location = MemoryLocation {
            page: params.memory_page_to_read,
            index: params.output_memory_offset,
        };

        let hash_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let hash_query = heaps.execute_partial_query(hash_query)?;
        let hash_value = hash_query.value;

        current_read_location.index += 1;
        let v_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let v_query = heaps.execute_partial_query(v_query)?;
        let v_value = v_query.value;

        current_read_location.index += 1;
        let r_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let r_query = heaps.execute_partial_query(r_query)?;
        let r_value = r_query.value;

        current_read_location.index += 1;
        let s_query = MemoryQuery {
            location: current_read_location,
            value: U256::zero(),
            value_is_pointer: false,
            rw_flag: false,
        };
        let s_query = heaps.execute_partial_query(s_query)?;
        let s_value = s_query.value;

        // read everything as bytes for ecrecover purposes
        let mut buffer = [0u8; 32];
        hash_value.to_big_endian(&mut buffer[..]);
        let hash = buffer;

        r_value.to_big_endian(&mut buffer[..]);
        let r_bytes = buffer;

        s_value.to_big_endian(&mut buffer[..]);
        let s_bytes = buffer;

        v_value.to_big_endian(&mut buffer[..]);
        let v = buffer[31];
        assert!(v == 0 || v == 1);

        let pk = ecrecover_inner(&hash, &r_bytes, &s_bytes, v);

        // here it may be possible to have non-recoverable k*G point, so can fail
        if let Ok(recovered_pubkey) = pk {
            let pk = k256::PublicKey::from(&recovered_pubkey);
            let affine_point = *pk.as_affine();
            use k256::elliptic_curve::sec1::ToEncodedPoint;
            let pk_bytes = affine_point.to_encoded_point(false);
            let pk_bytes_ref: &[u8] = pk_bytes.as_ref();
            assert_eq!(pk_bytes_ref.len(), 65);
            debug_assert_eq!(pk_bytes_ref[0], 0x04);
            let address_hash = sha3::Keccak256::digest(&pk_bytes_ref[1..]);

            let mut address = [0u8; 32];
            let hash_ref: &[u8] = address_hash.as_ref();
            address[12..].copy_from_slice(&hash_ref[12..]);

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
            heaps.execute_partial_query(ok_or_err_query)?;

            write_location.index += 1;
            let result = U256::from_big_endian(&address);
            let result_query = MemoryQuery {
                location: write_location,
                value: result,
                value_is_pointer: false,
                rw_flag: true,
            };
            heaps.execute_partial_query(result_query)?;
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
            heaps.execute_partial_query(ok_or_err_query)?;

            write_location.index += 1;
            let empty_result = U256::zero();
            let result_query = MemoryQuery {
                location: write_location,
                value: empty_result,
                value_is_pointer: false,
                rw_flag: true,
            };
            heaps.execute_partial_query(result_query)?;
        }
        Ok(())
    }
}

pub fn ecrecover_inner(
    digest: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
    rec_id: u8,
) -> Result<VerifyingKey, ()> {
    use k256::ecdsa::{RecoveryId, Signature};
    // r, s
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(r);
    signature[32..].copy_from_slice(s);
    // we expect pre-validation, so this check always works
    let signature = Signature::try_from(&signature[..]).map_err(|_| ())?;
    let recid = RecoveryId::try_from(rec_id).unwrap();

    recover_no_malleability_check(digest, signature, recid)
}

fn recover_no_malleability_check(
    digest: &[u8; 32],
    signature: k256::ecdsa::Signature,
    recovery_id: k256::ecdsa::RecoveryId,
) -> Result<VerifyingKey, ()> {
    use k256::ecdsa::hazmat::bits2field;
    use k256::elliptic_curve::bigint::CheckedAdd;
    use k256::elliptic_curve::generic_array::GenericArray;
    use k256::elliptic_curve::ops::Invert;
    use k256::elliptic_curve::ops::LinearCombination;
    use k256::elliptic_curve::ops::Reduce;
    use k256::elliptic_curve::point::DecompressPoint;
    use k256::elliptic_curve::Curve;
    use k256::elliptic_curve::FieldBytesEncoding;
    use k256::elliptic_curve::PrimeField;
    use k256::AffinePoint;
    use k256::ProjectivePoint;
    use k256::Scalar;

    let (r, s) = signature.split_scalars();
    let z = <Scalar as Reduce<k256::U256>>::reduce_bytes(
        &bits2field::<k256::Secp256k1>(digest).map_err(|_| ())?,
    );

    let mut r_bytes: GenericArray<u8, <k256::Secp256k1 as Curve>::FieldBytesSize> = r.to_repr();
    if recovery_id.is_x_reduced() {
        match Option::<k256::U256>::from(
            <k256::U256 as FieldBytesEncoding<k256::Secp256k1>>::decode_field_bytes(&r_bytes)
                .checked_add(&k256::Secp256k1::ORDER),
        ) {
            Some(restored) => {
                r_bytes = <k256::U256 as FieldBytesEncoding<k256::Secp256k1>>::encode_field_bytes(
                    &restored,
                )
            }
            // No reduction should happen here if r was reduced
            None => return Err(()),
        };
    }

    #[allow(non_snake_case)]
    let R = AffinePoint::decompress(&r_bytes, u8::from(recovery_id.is_y_odd()).into());

    if R.is_none().into() {
        return Err(());
    }

    #[allow(non_snake_case)]
    let R = ProjectivePoint::from(R.unwrap());
    let r_inv: Scalar = *r.invert();
    let u1 = -(r_inv * z);
    let u2 = r_inv * *s;
    let pk = ProjectivePoint::lincomb(&ProjectivePoint::GENERATOR, &u1, &R, &u2);
    let vk = VerifyingKey::from_affine(pk.into()).map_err(|_| ())?;

    // Ensure signature verifies with the recovered key
    let field = bits2field::<k256::Secp256k1>(digest).map_err(|_| ())?;
    // here we actually skip a high-s check (that should never be there at the first place and should be checked by caller)
    k256::ecdsa::hazmat::verify_prehashed(&vk.as_affine().into(), &field, &signature)
        .map_err(|_| ())?;

    Ok(vk)
}

pub fn ecrecover_function(abi: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
    let mut processor = ECRecoverPrecompile;
    processor.execute_precompile(abi, heaps)
}
