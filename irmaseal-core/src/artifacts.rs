//! This module implements constant-time serialization and deserialization for the USK and MPK
//! suitable for use in a HTTP API.  MPK serialization does not have to be constant-time, but this
//! way we only require one dependency.

use crate::util::open_ct;
use base64ct::{Base64, Encoding};
use ibe::{
    kem::{cgw_kv::CGWKV, IBKEM},
    Compress,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Computes the byte length of raw bytes encoded in (padded) b64.
// We use this to preallocate a buffer to encode into.
const fn b64len(raw_len: usize) -> usize {
    // use next line when unwrap() becomes stable as a const fn:
    // .checked_mul(4).unwrap()
    // this will cause a compiler error when the multiplication overflows,
    // making this function "safe" for all input.
    (((raw_len - 1) / 3) + 1) * 4
}

#[cfg(feature = "v1")]
use ibe::kem::kiltz_vahlis_one::KV1;

/// An IRMAseal public key for a system, as generated by the Private Key Generator (PKG).
#[derive(Debug, Clone, Copy)]
pub struct PublicKey<K: IBKEM>(pub K::Pk);

/// An IRMAseal user private key, as generated by the Private Key Generator (PKG).
#[derive(Debug)]
pub struct UserSecretKey<K: IBKEM>(pub K::Usk);

// Note: We cannot make these implementations generic parameter over the scheme parameter because
// of this constant expression depending on a generic parameter, see
// https://github.com/rust-lang/rust/issues/68436.
//
// For now, the solutions are these deserialize impl macros, creating encoding/decoding buffer for
// each scheme specifically.

macro_rules! impl_deserialize_pk {
    ($scheme: ident) => {
        impl<'de> Deserialize<'de> for PublicKey<$scheme> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = <&'de str>::deserialize(deserializer)?;

                let mut dec_buf = [0u8; $scheme::PK_BYTES];
                Base64::decode(s, &mut dec_buf)
                    .map_err(|_e| serde::de::Error::custom("base64ct decoding error"))?;

                let pk = open_ct(<$scheme as IBKEM>::Pk::from_bytes(&dec_buf))
                    .ok_or(serde::de::Error::custom("not a public key"))?;

                Ok(PublicKey(pk))
            }
        }
    };
}

macro_rules! impl_deserialize_usk {
    ($scheme: ident) => {
        impl<'de> Deserialize<'de> for UserSecretKey<$scheme> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = <&'de str>::deserialize(deserializer)?;

                let mut dec_buf = [0u8; $scheme::USK_BYTES];
                Base64::decode(s, &mut dec_buf)
                    .map_err(|_e| serde::de::Error::custom("base64ct decoding error"))?;

                let usk = open_ct(<$scheme as IBKEM>::Usk::from_bytes(&dec_buf))
                    .ok_or(serde::de::Error::custom("not a user secret key"))?;

                Ok(UserSecretKey(usk))
            }
        }
    };
}

macro_rules! impl_serialize_pk {
    ($scheme: ident) => {
        impl Serialize for PublicKey<$scheme> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut enc_buf = [0u8; b64len($scheme::PK_BYTES)];

                let encoded =
                    Base64::encode(self.0.to_bytes().as_ref(), &mut enc_buf).map_err(|e| {
                        serde::ser::Error::custom(format!("base64ct serialization error: {}", e))
                    })?;

                serializer.serialize_str(encoded)
            }
        }
    };
}

macro_rules! impl_serialize_usk {
    ($scheme: ident) => {
        impl Serialize for UserSecretKey<$scheme> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut enc_buf = [0u8; b64len($scheme::USK_BYTES)];

                let encoded =
                    Base64::encode(self.0.to_bytes().as_ref(), &mut enc_buf).map_err(|e| {
                        serde::ser::Error::custom(format!("base64ct serialization error: {}", e))
                    })?;

                serializer.serialize_str(encoded)
            }
        }
    };
}

impl_serialize_pk!(CGWKV);
impl_serialize_usk!(CGWKV);

impl_deserialize_pk!(CGWKV);
impl_deserialize_usk!(CGWKV);

#[cfg(feature = "v1")]
impl_serialize_pk!(KV1);

#[cfg(feature = "v1")]
impl_serialize_usk!(KV1);

#[cfg(feature = "v1")]
impl_deserialize_pk!(KV1);

#[cfg(feature = "v1")]
impl_deserialize_usk!(KV1);

#[cfg(test)]
mod tests {
    use super::*;
    use ibe::Derive;

    #[test]
    fn test_eq_enc_dec() {
        let mut rng = rand::thread_rng();
        let (mpk, msk) = ibe::kem::cgw_kv::CGWKV::setup(&mut rng);
        let wrapped_pk = PublicKey::<CGWKV>(mpk);

        let pk_encoded = serde_json::to_string(&wrapped_pk).unwrap();
        let pk_decoded: PublicKey<CGWKV> = serde_json::from_str(&pk_encoded).unwrap();

        assert_eq!(&wrapped_pk.0.to_bytes(), &pk_decoded.0.to_bytes());

        let id = <CGWKV as IBKEM>::Id::derive_str("test");
        let usk = CGWKV::extract_usk(Some(&mpk), &msk, &id, &mut rng);
        let wrapped_usk = UserSecretKey::<CGWKV>(usk);

        let usk_encoded = serde_json::to_string(&wrapped_usk).unwrap();
        let usk_decoded: UserSecretKey<CGWKV> = serde_json::from_str(&usk_encoded).unwrap();

        assert_eq!(&wrapped_usk.0.to_bytes(), &usk_decoded.0.to_bytes());
    }

    #[test]
    #[cfg(feature = "v1")]
    fn test_eq_enc_dec2() {
        use ibe::kem::kiltz_vahlis_one::KV1;

        let mut rng = rand::thread_rng();
        let (mpk, msk) = ibe::kem::kiltz_vahlis_one::KV1::setup(&mut rng);
        let wrapped_pk = PublicKey::<KV1>(mpk);

        let pk_encoded = serde_json::to_string(&wrapped_pk).unwrap();
        let pk_decoded: PublicKey<KV1> = serde_json::from_str(&pk_encoded).unwrap();

        assert_eq!(&wrapped_pk.0.to_bytes(), &pk_decoded.0.to_bytes());

        let id = <KV1 as IBKEM>::Id::derive_str("test");
        let usk = KV1::extract_usk(Some(&mpk), &msk, &id, &mut rng);
        let wrapped_usk = UserSecretKey::<KV1>(usk);

        let usk_encoded = serde_json::to_string(&wrapped_usk).unwrap();
        let usk_decoded: UserSecretKey<KV1> = serde_json::from_str(&usk_encoded).unwrap();

        assert_eq!(&wrapped_usk.0.to_bytes(), &usk_decoded.0.to_bytes());
    }
}
