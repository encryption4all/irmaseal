use crate::metadata::*;
use crate::util::generate_iv;
use crate::*;

use ibe::kem::cgw_fo::CGWFO;
use ibe::kem::IBKEM;
use rand::{CryptoRng, Rng};
use std::io::Write;

impl Metadata {
    /// Create a new metadata.
    ///
    /// Consumes the policies.
    pub fn new<R: Rng + CryptoRng>(
        pk: &PublicKey<CGWFO>,
        policies: &BTreeMap<String, Policy>,
        rng: &mut R,
    ) -> Result<(Self, SharedSecret), Error> {
        // Generate a bunch of default ciphertexts.
        let mut cts = vec![<CGWFO as IBKEM>::Ct::default(); policies.len()];

        // Map policies to IBE identities.
        let ibe_ids = policies
            .values()
            .map(|p| p.derive::<CGWFO>())
            .collect::<Result<Vec<<CGWFO as IBKEM>::Id>, _>>()?;

        // Map to references of IBE identities.
        let refs: Vec<&<CGWFO as IBKEM>::Id> = ibe_ids.iter().collect();

        // Generate the shared secret and ciphertexts.
        let ss = CGWFO::multi_encaps(&pk.0, &refs[..], rng, &mut cts[..]).unwrap();

        // Generate all RecipientInfo's.
        let recipient_info: BTreeMap<String, RecipientInfo> = policies
            .into_iter()
            .zip(cts.iter())
            .map(|((rid, policy), ct)| {
                (
                    rid.clone(),
                    RecipientInfo {
                        policy: policy.to_hidden(),
                        ct: ct.to_bytes(),
                    },
                )
            })
            .collect();

        Ok((
            Metadata {
                policies: recipient_info,
                iv: generate_iv(rng),
                chunk_size: SYMMETRIC_CRYPTO_DEFAULT_CHUNK,
            },
            ss,
        ))
    }

    /// Writes binary msgPack format into a std::io::Writer.
    ///
    /// Internally uses the "named" convention, which preserves field names.
    /// Fields names are shortened to limit overhead:
    /// `rs`: sequence of serialized `RecipientInfo`s,
    ///     `id`: serialized `RecipientIdentifier`,
    ///     `p`: serialized `HiddenPolicy`:
    ///     `ct`: associated ciphertext with this policy,
    /// `iv`: 16-byte initialization vector,
    /// `cs`: chunk size in bytes used in the symmetrical encryption.
    pub fn msgpack_into<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        w.write_all(&PRELUDE)?;
        w.write_all(&VERSION_V2.to_be_bytes())?;

        // For this to work, we need know the length of the metadata in advance.
        // For now, buffer it and determine the length.
        // TODO: could optimize this, or at least use a max capacity.
        let mut meta_vec = Vec::new();
        let mut serializer = rmp_serde::encode::Serializer::new(&mut meta_vec).with_struct_map();

        self.serialize(&mut serializer)
            .map_err(|_e| Error::ConstraintViolation)?;

        w.write_all(
            &u32::try_from(meta_vec.len())
                .map_err(|_e| Error::ConstraintViolation)?
                .to_be_bytes(),
        )?;
        w.write_all(&meta_vec)?;

        Ok(())
    }

    /// Writes to a pretty json string.
    ///
    /// Should only be used for small metadata or development purposes.
    pub fn to_json_string(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(&self).or(Err(Error::FormatViolation))?)
    }
}
