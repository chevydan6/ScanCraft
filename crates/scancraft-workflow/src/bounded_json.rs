//! Bounded typed JSON arrays shared by read-only diagnostic loaders.
use serde::{
    Deserialize,
    de::{self, SeqAccess, Visitor},
};

// Typed summaries skip unused observation fields instead of allocating a full
// Value tree. Item limits are enforced during deserialization, before growth.
pub(crate) struct Items<T, const MAX: usize>(pub(crate) Vec<T>);
impl<'de, T: Deserialize<'de>, const MAX: usize> Deserialize<'de> for Items<T, MAX> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ItemVisitor<T, const MAX: usize>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for ItemVisitor<T, MAX> {
            type Value = Items<T, MAX>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "an array of at most {MAX} items")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(value) = seq.next_element()? {
                    if items.len() == MAX {
                        return Err(de::Error::custom("candidate array item limit exceeded"));
                    }
                    items.push(value);
                }
                Ok(Items(items))
            }
        }
        deserializer.deserialize_seq(ItemVisitor::<T, MAX>(std::marker::PhantomData))
    }
}
