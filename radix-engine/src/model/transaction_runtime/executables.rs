use crate::engine::*;
use crate::types::*;
use crate::wasm::WasmEngine;
use radix_engine_interface::api::types::*;
use radix_engine_interface::api::EngineApi;
use radix_engine_interface::crypto::hash;

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub enum TransactionRuntimeError {
    OutOfUUid,
}

impl ExecutableInvocation for TransactionRuntimeGetHashInvocation {
    type Exec = Self;

    fn resolve<D: ResolverApi>(
        self,
        _deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let actor = ResolvedActor::method(
            NativeFn::TransactionRuntime(TransactionRuntimeFn::Get),
            ResolvedReceiver::new(RENodeId::TransactionRuntime(self.receiver)),
        );
        let call_frame_update = CallFrameUpdate::empty();

        Ok((actor, call_frame_update, self))
    }
}

impl Executor for TransactionRuntimeGetHashInvocation {
    type Output = Hash;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError>,
    {
        let offset =
            SubstateOffset::TransactionRuntime(TransactionRuntimeOffset::TransactionRuntime);
        let node_id = RENodeId::TransactionRuntime(self.receiver);
        let handle = api.lock_substate(node_id, offset, LockFlags::read_only())?;
        let substate = api.get_ref(handle)?;
        let transaction_runtime_substate = substate.transaction_runtime();
        Ok((
            transaction_runtime_substate.hash.clone(),
            CallFrameUpdate::empty(),
        ))
    }
}

impl ExecutableInvocation for TransactionRuntimeGenerateUuidInvocation {
    type Exec = Self;

    fn resolve<D: ResolverApi>(
        self,
        _deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let actor = ResolvedActor::method(
            NativeFn::TransactionRuntime(TransactionRuntimeFn::GenerateUuid),
            ResolvedReceiver::new(RENodeId::TransactionRuntime(self.receiver)),
        );

        let call_frame_update = CallFrameUpdate::empty();

        Ok((actor, call_frame_update, self))
    }
}

impl Executor for TransactionRuntimeGenerateUuidInvocation {
    type Output = u128;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError>,
    {
        let offset =
            SubstateOffset::TransactionRuntime(TransactionRuntimeOffset::TransactionRuntime);
        let node_id = RENodeId::TransactionRuntime(self.receiver);
        let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;
        let mut substate_mut = api.get_ref_mut(handle)?;
        let transaction_hash_substate = substate_mut.transaction_runtime();

        if transaction_hash_substate.next_id == u32::MAX {
            return Err(RuntimeError::ApplicationError(
                ApplicationError::TransactionRuntimeError(TransactionRuntimeError::OutOfUUid),
            ));
        }

        let mut data = transaction_hash_substate.hash.to_vec();
        data.extend(transaction_hash_substate.next_id.to_le_bytes());
        let uuid = u128::from_le_bytes(hash(data).lower_16_bytes()); // TODO: Remove hash

        transaction_hash_substate.next_id = transaction_hash_substate.next_id + 1;

        Ok((uuid, CallFrameUpdate::empty()))
    }
}