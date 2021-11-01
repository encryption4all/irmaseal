use crate::util::open_ct;
use ibe::{
    kem::{cgw_fo::CGWFO, IBKEM},
    Compress,
};

#[cfg(feature = "v1")]
use ibe::kem::kiltz_vahlis_one::KV1;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An IRMAseal public key for a system, as generated by the Private Key Generator (PKG).
#[derive(Debug, Clone, Copy)]
pub struct PublicKey<K: IBKEM>(pub K::Pk);

/// An IRMAseal user private key, as generated by the Private Key Generator (PKG).
#[derive(Debug)]
pub struct UserSecretKey<K: IBKEM>(pub K::Usk);

// Note:
// We cannot make this implementation have a generic parameter because
// of this constant expression depending on a generic parameter, see
// https://github.com/rust-lang/rust/issues/68436.
//
// For now, the solutions are these deserialize impl macros.

macro_rules! impl_deserialize_pk {
    ($scheme: ident) => {
        /// Deserialize from a base64 encoded waters byte representation.
        impl<'de> Deserialize<'de> for PublicKey<$scheme> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = <&'de str>::deserialize(deserializer)?;
                let mut b = [0u8; $scheme::PK_BYTES];
                base64::decode_config_slice(s, base64::STANDARD, &mut b)
                    .map_err(|_e| serde::de::Error::custom("decoding error"))?;
                let pk = open_ct(<$scheme as IBKEM>::Pk::from_bytes(&b))
                    .ok_or_else(|| serde::de::Error::custom("not a public key"))?;

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
                let mut b = [0u8; $scheme::USK_BYTES];
                base64::decode_config_slice(s, base64::STANDARD, &mut b)
                    .map_err(|_e| serde::de::Error::custom("decoding error"))?;
                let usk = open_ct(<$scheme as IBKEM>::Usk::from_bytes(&b))
                    .ok_or_else(|| serde::de::Error::custom("not a user secret key"))?;

                Ok(UserSecretKey(usk))
            }
        }
    };
}

/// Serialize to a base64 encoded byte representation.
impl<K: IBKEM> Serialize for PublicKey<K> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&base64::encode(self.0.to_bytes().as_ref()))
    }
}

/// Serialize to a base64 encoded waters byte representation.
impl<K: IBKEM> Serialize for UserSecretKey<K> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&base64::encode(self.0.to_bytes().as_ref()))
    }
}

impl_deserialize_pk!(CGWFO);
impl_deserialize_usk!(CGWFO);

#[cfg(feature = "v1")]
impl_deserialize_pk!(KV1);

#[cfg(feature = "v1")]
impl_deserialize_usk!(KV1);
