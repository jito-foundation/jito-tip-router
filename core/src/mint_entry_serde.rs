// mint_entry_serde.rs
use std::{fmt, marker::PhantomData};

use serde::{
    de::{Error, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::tracked_mints::MintEntry;

// Helper struct for serialization
#[derive(Serialize, Deserialize)]
struct SerializableMintEntry {
    st_mint: String, // Pubkey as base58 string
    vault_index: u64,
    #[serde(with = "serde_with::As::<serde_with::Bytes>")]
    reserved: [u8; 32],
}

impl From<&MintEntry> for SerializableMintEntry {
    fn from(entry: &MintEntry) -> Self {
        Self {
            st_mint: entry.st_mint.to_string(),
            vault_index: entry.vault_index.into(),
            reserved: entry.reserved,
        }
    }
}

impl From<SerializableMintEntry> for MintEntry {
    fn from(entry: SerializableMintEntry) -> Self {
        Self {
            st_mint: entry.st_mint.parse().unwrap_or_default(),
            vault_index: entry.vault_index.into(),
            reserved: entry.reserved,
        }
    }
}
pub struct MintEntryArraySerializer;

impl MintEntryArraySerializer {
    pub fn serialize<S>(entries: &[MintEntry; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(64))?;
        for entry in entries.iter() {
            seq.serialize_element(&SerializableMintEntry::from(entry))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[MintEntry; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MintEntryArrayVisitor(PhantomData<MintEntry>);

        impl<'de> Visitor<'de> for MintEntryArrayVisitor {
            type Value = [MintEntry; 64];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an array of 64 MintEntry")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut entries = [MintEntry::default(); 64];
                for i in 0..64 {
                    entries[i] = seq
                        .next_element()?
                        .ok_or_else(|| Error::invalid_length(i, &self))?;
                }
                Ok(entries)
            }
        }

        deserializer.deserialize_seq(MintEntryArrayVisitor(PhantomData))
    }
}
