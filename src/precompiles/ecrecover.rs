use crate::eravm_error::{EraVmError, PrecompileError};
use k256::{
    ecdsa::{hazmat::bits2field, RecoveryId, Signature, VerifyingKey},
    elliptic_curve::{
        bigint::CheckedAdd,
        generic_array::GenericArray,
        ops::{Invert, LinearCombination, Reduce},
        point::DecompressPoint,
        Curve, FieldBytesEncoding, PrimeField,
    },
    AffinePoint, ProjectivePoint, Scalar,
};
use sha3::{Digest, Keccak256};
use u256::U256;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ECRecoverPrecompile;

impl Precompile for ECRecoverPrecompile {
    fn execute_precompile(&mut self, query: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
        let precompile_call_params = query;
        let params = precompile_abi_in_log(precompile_call_params);
        let read_heap_idx = params.memory_page_to_read;
        let write_heap_idx = params.memory_page_to_write;
        let mut addr = params.output_memory_offset;

        let (hash_value, _) =
            heaps.read(params.memory_page_to_read, params.output_memory_offset)?;
        let (v_value, _) = heaps.read(read_heap_idx, addr + 1)?;
        let (r_value, _) = heaps.read(read_heap_idx, addr + 2)?;
        let (s_value, _) = heaps.read(read_heap_idx, addr + 3)?;
        addr = addr + 3;

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

        if v != 0 && v != 1 {
            return Err(PrecompileError::EcRecoverInvalidByte.into());
        }

        let pk = ecrecover_inner(&hash, &r_bytes, &s_bytes, v);
        // here it may be possible to have non-recoverable k*G point, so can fail
        let (marker, result) = match pk {
            Ok(recovered_pubkey) => (
                U256::one(),
                U256::from_big_endian(&get_address_from_pk(recovered_pubkey)?),
            ),
            _ => (U256::zero(), U256::zero()),
        };
        heaps.write(write_heap_idx, addr, marker)?;
        heaps.write(write_heap_idx, addr + 1, result)?;
        Ok(())
    }
}

// Recovers the public key from a given digest and signature.
pub fn ecrecover_inner(
    digest: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
    rec_id: u8,
) -> Result<VerifyingKey, ()> {
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

    let y = AffinePoint::decompress(&r_bytes, u8::from(recovery_id.is_y_odd()).into());

    if y.is_none().into() {
        return Err(());
    }

    let y = ProjectivePoint::from(y.unwrap());
    let r_inv: Scalar = *r.invert();
    let u1 = -(r_inv * z);
    let u2 = r_inv * *s;
    let pk = ProjectivePoint::lincomb(&ProjectivePoint::GENERATOR, &u1, &y, &u2);
    let vk = VerifyingKey::from_affine(pk.into()).map_err(|_| ())?;

    // Ensure signature verifies with the recovered key
    let field = bits2field::<k256::Secp256k1>(digest).map_err(|_| ())?;
    // here we actually skip a high-s check (that should never be there at the first place and should be checked by caller)
    k256::ecdsa::hazmat::verify_prehashed(&vk.as_affine().into(), &field, &signature)
        .map_err(|_| ())?;

    Ok(vk)
}

fn get_address_from_pk(pk: VerifyingKey) -> Result<[u8; 32], EraVmError> {
    let pk = k256::PublicKey::from(pk);
    let affine_point = *pk.as_affine();
    use k256::elliptic_curve::sec1::ToEncodedPoint;
    let pk_bytes = affine_point.to_encoded_point(false);
    let pk_bytes_ref: &[u8] = pk_bytes.as_ref();
    if pk_bytes_ref.len() != 65 && pk_bytes_ref[0] != 0x04 {
        return Err(EraVmError::OutOfGas);
    }
    let address_hash = Keccak256::digest(&pk_bytes_ref[1..]);

    let mut address = [0u8; 32];
    let hash_ref: &[u8] = address_hash.as_ref();
    address[12..].copy_from_slice(&hash_ref[12..]);

    Ok(address)
}

pub fn ecrecover_function(abi: U256, heaps: &mut Heaps) -> Result<(), EraVmError> {
    ECRecoverPrecompile.execute_precompile(abi, heaps)
}
