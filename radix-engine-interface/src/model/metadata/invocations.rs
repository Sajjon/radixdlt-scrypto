use sbor::rust::fmt::Debug;

use crate::api::{api::*, types::*};
use crate::scrypto;
use crate::wasm::*;

#[derive(Debug)]
#[scrypto(TypeId, Encode, Decode)]
pub struct MetadataSetInvocation {
    pub receiver: RENodeId,
    pub key: String,
    pub value: String,
}

impl Invocation for MetadataSetInvocation {
    type Output = ();
}

impl ScryptoNativeInvocation for MetadataSetInvocation {
    type ScryptoOutput = ();
}

impl Into<NativeFnInvocation> for MetadataSetInvocation {
    fn into(self) -> NativeFnInvocation {
        NativeFnInvocation::Method(NativeMethodInvocation::Metadata(
            MetadataMethodInvocation::Set(self),
        ))
    }
}
