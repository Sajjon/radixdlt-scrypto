use crate::rust::collections::*;
use crate::rust::hash::Hash;
use crate::rust::ptr::copy;
use crate::rust::vec::Vec;
use crate::type_id::*;
use crate::*;

impl<X: CustomTypeId, E: Encoder<X>, T: Encode<X, E> + TypeId<X>> Encode<X, E> for Vec<T> {
    #[inline]
    fn encode_type_id(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(Self::type_id())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.as_slice().encode_body(encoder)?;
        Ok(())
    }
}

impl<X: CustomTypeId, E: Encoder<X>, T: Encode<X, E> + TypeId<X>> Encode<X, E> for BTreeSet<T> {
    #[inline]
    fn encode_type_id(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(Self::type_id())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(T::type_id())?;
        encoder.write_size(self.len())?;
        for v in self {
            encoder.encode_deeper_body(v)?;
        }
        Ok(())
    }
}

impl<X: CustomTypeId, E: Encoder<X>, T: Encode<X, E> + TypeId<X> + Ord + Hash> Encode<X, E>
    for HashSet<T>
{
    #[inline]
    fn encode_type_id(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(Self::type_id())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(T::type_id())?;
        encoder.write_size(self.len())?;
        let set: BTreeSet<&T> = self.iter().collect();
        for v in set {
            encoder.encode_deeper_body(v)?;
        }
        Ok(())
    }
}

#[cfg(feature = "indexmap")]
impl<X: CustomTypeId, E: Encoder<X>, T: Encode<X, E> + TypeId<X> + Hash> Encode<X, E>
    for indexmap::IndexSet<T>
{
    #[inline]
    fn encode_type_id(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(Self::type_id())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(T::type_id())?;
        encoder.write_size(self.len())?;
        for v in self {
            encoder.encode_deeper_body(v)?;
        }
        Ok(())
    }
}

impl<X: CustomTypeId, E: Encoder<X>, K: Encode<X, E>, V: Encode<X, E>> Encode<X, E>
    for BTreeMap<K, V>
{
    #[inline]
    fn encode_type_id(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(Self::type_id())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(<(K, V)>::type_id())?;
        encoder.write_size(self.len())?;
        for (k, v) in self {
            encoder.encode_deeper_body(&(k, v))?;
        }
        Ok(())
    }
}

impl<X: CustomTypeId, E: Encoder<X>, K: Encode<X, E> + Ord + Hash, V: Encode<X, E>> Encode<X, E>
    for HashMap<K, V>
{
    #[inline]
    fn encode_type_id(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(Self::type_id())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(<(K, V)>::type_id())?;
        encoder.write_size(self.len())?;
        let keys: BTreeSet<&K> = self.keys().collect();
        for key in keys {
            encoder.encode_deeper_body(&(key, self.get(key).unwrap()))?;
        }
        Ok(())
    }
}

#[cfg(feature = "indexmap")]
impl<X: CustomTypeId, E: Encoder<X>, K: Encode<X, E> + Ord + Hash, V: Encode<X, E>> Encode<X, E>
    for indexmap::IndexMap<K, V>
{
    #[inline]
    fn encode_type_id(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(Self::type_id())
    }

    #[inline]
    fn encode_body(&self, encoder: &mut E) -> Result<(), EncodeError> {
        encoder.write_type_id(<(K, V)>::type_id())?;
        encoder.write_size(self.len())?;
        for (key, value) in self {
            encoder.encode_deeper_body(&(key, value))?;
        }
        Ok(())
    }
}

impl<X: CustomTypeId, D: Decoder<X>, T: Decode<X, D> + TypeId<X>> Decode<X, D> for Vec<T> {
    #[inline]
    fn decode_body_with_type_id(
        decoder: &mut D,
        type_id: SborTypeId<X>,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let element_type_id = decoder.read_and_check_type_id(T::type_id())?;
        let len = decoder.read_size()?;

        if T::type_id() == SborTypeId::U8 || T::type_id() == SborTypeId::I8 {
            let slice = decoder.read_slice(len)?; // length is checked here
            let mut result = Vec::<T>::with_capacity(len);
            unsafe {
                copy(slice.as_ptr(), result.as_mut_ptr() as *mut u8, slice.len());
                result.set_len(slice.len());
            }
            Ok(result)
        } else {
            let mut result = Vec::<T>::with_capacity(if len <= 1024 { len } else { 1024 });
            for _ in 0..len {
                result.push(decoder.decode_deeper_body_with_type_id(element_type_id)?);
            }
            Ok(result)
        }
    }
}

impl<X: CustomTypeId, D: Decoder<X>, T: Decode<X, D> + TypeId<X> + Ord> Decode<X, D>
    for BTreeSet<T>
{
    #[inline]
    fn decode_body_with_type_id(
        decoder: &mut D,
        type_id: SborTypeId<X>,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let elements: Vec<T> = Vec::<T>::decode_body_with_type_id(decoder, type_id)?;
        Ok(elements.into_iter().collect())
    }
}

impl<X: CustomTypeId, D: Decoder<X>, T: Decode<X, D> + TypeId<X> + Hash + Eq> Decode<X, D>
    for HashSet<T>
{
    #[inline]
    fn decode_body_with_type_id(
        decoder: &mut D,
        type_id: SborTypeId<X>,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let elements: Vec<T> = Vec::<T>::decode_body_with_type_id(decoder, type_id)?;
        Ok(elements.into_iter().collect())
    }
}

#[cfg(feature = "indexmap")]
impl<X: CustomTypeId, D: Decoder<X>, T: Decode<X, D> + TypeId<X> + Hash + Eq> Decode<X, D>
    for IndexSet<T>
{
    #[inline]
    fn decode_body_with_type_id(
        decoder: &mut D,
        type_id: SborTypeId<X>,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let element_type_id = decoder.read_and_check_type_id(T::type_id())?;
        let len = decoder.read_size()?;
        let mut result = IndexSet::<T>::with_capacity(if len <= 1024 { len } else { 1024 });
        for _ in 0..len {
            result.insert(decoder.decode_deeper_body_with_type_id(element_type_id)?);
        }
        Ok(result)
    }
}

impl<X: CustomTypeId, D: Decoder<X>, K: Decode<X, D> + Ord, V: Decode<X, D>> Decode<X, D>
    for BTreeMap<K, V>
{
    #[inline]
    fn decode_body_with_type_id(
        decoder: &mut D,
        type_id: SborTypeId<X>,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let elements = Vec::<(K, V)>::decode_body_with_type_id(decoder, type_id)?;
        Ok(elements.into_iter().collect())
    }
}

impl<X: CustomTypeId, D: Decoder<X>, K: Decode<X, D> + Hash + Eq, V: Decode<X, D>> Decode<X, D>
    for HashMap<K, V>
{
    #[inline]
    fn decode_body_with_type_id(
        decoder: &mut D,
        type_id: SborTypeId<X>,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let elements: Vec<(K, V)> = Vec::<(K, V)>::decode_body_with_type_id(decoder, type_id)?;
        Ok(elements.into_iter().collect())
    }
}

#[cfg(feature = "indexmap")]
impl<X: CustomTypeId, D: Decoder<X>, K: Decode<X, D> + Hash + Eq, V: Decode<X, D>> Decode<X, D>
    for indexmap::IndexMap<K, V>
{
    #[inline]
    fn decode_body_with_type_id(
        decoder: &mut D,
        type_id: SborTypeId<X>,
    ) -> Result<Self, DecodeError> {
        decoder.check_preloaded_type_id(type_id, Self::type_id())?;
        let elements: Vec<(K, V)> = Vec::<(K, V)>::decode_body_with_type_id(decoder, type_id)?;
        Ok(elements.into_iter().collect())
    }
}

#[cfg(feature = "schema")]
pub use schema::*;

#[cfg(feature = "schema")]
mod schema {
    use super::*;

    use_same_generic_vec_schema!(T, Vec<T>, [T]);

    impl<C: CustomTypeSchema, T: Schema<C> + TypeId<C::CustomTypeId>> Schema<C> for BTreeSet<T> {
        const SCHEMA_TYPE_REF: GlobalTypeRef = GlobalTypeRef::complex("Set", &[T::SCHEMA_TYPE_REF]);

        fn get_local_type_data() -> Option<LocalTypeData<C, GlobalTypeRef>> {
            Some(LocalTypeData {
                schema: TypeSchema::Array {
                    element_sbor_type_id: T::type_id().as_u8(),
                    element_type: T::SCHEMA_TYPE_REF,
                    length_validation: LengthValidation::none(),
                },
                naming: TypeNaming::named_no_child_names("Set"),
            })
        }

        fn add_all_dependencies(aggregator: &mut SchemaAggregator<C>) {
            aggregator.add_child_type_and_descendents::<T>();
        }
    }

    use_same_generic_vec_schema!(T, HashSet<T>, BTreeSet<T>);
    #[cfg(feature = "indexmap")]
    use_same_generic_vec_schema!(T, IndexSet<T>, BTreeSet<T>);

    impl<C: CustomTypeSchema, K: Schema<C>, V: Schema<C>> Schema<C> for BTreeMap<K, V> {
        const SCHEMA_TYPE_REF: GlobalTypeRef =
            GlobalTypeRef::complex("Map", &[K::SCHEMA_TYPE_REF, V::SCHEMA_TYPE_REF]);

        fn get_local_type_data() -> Option<LocalTypeData<C, GlobalTypeRef>> {
            Some(LocalTypeData {
                schema: TypeSchema::Array {
                    element_sbor_type_id: <(K, V) as TypeId<C::CustomTypeId>>::type_id().as_u8(),
                    element_type: <(K, V)>::SCHEMA_TYPE_REF,
                    length_validation: LengthValidation::none(),
                },
                naming: TypeNaming::named_no_child_names("Map"),
            })
        }

        fn add_all_dependencies(aggregator: &mut SchemaAggregator<C>) {
            aggregator.add_child_type_and_descendents::<(K, V)>();
        }
    }

    use_same_double_generic_schema!(K, V, HashMap<K, V>, BTreeMap<K, V>);
    #[cfg(feature = "indexmap")]
    use_same_double_generic_schema!(K, V, IndexMap<K, V>, BTreeMap<K, V>);
}
