use crate::kernel::kernel_api::{KernelInternalApi, KernelInvocation};
use crate::system::module::SystemModule;
use crate::system::system_callback::SystemConfig;
use crate::system::system_callback_api::SystemCallbackObject;
use crate::track::interface::{NodeSubstates, StoreAccess};
use crate::types::*;
use crate::{errors::RuntimeError, errors::SystemModuleError, kernel::kernel_api::KernelApi};
use crate::kernel::kernel_callback_api::CreateNodeEvent;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum TransactionLimitsError {
    MaxSubstateSizeExceeded(usize),
    MaxInvokePayloadSizeExceeded(usize),
    MaxCallDepthLimitReached,
    TooManyEntriesInTrack,
    LogSizeTooLarge { actual: usize, max: usize },
    EventSizeTooLarge { actual: usize, max: usize },
    PanicMessageSizeTooLarge { actual: usize, max: usize },
    TooManyLogs,
    TooManyEvents,
}

pub struct TransactionLimitsConfig {
    pub max_number_of_substates_in_track: usize,
    pub max_number_of_substates_in_heap: usize, // FIXME: enforce this limits in heap!
    pub max_substate_size: usize,
    pub max_invoke_payload_size: usize,
    pub max_event_size: usize,
    pub max_log_size: usize,
    pub max_panic_message_size: usize,
    pub max_number_of_logs: usize,
    pub max_number_of_events: usize,
}

/// Tracks and verifies transaction limits during transactino execution,
/// if exceeded breaks execution with appropriate error.
/// Default limits values are defined in radix-engine-common/constants.
/// Stores boundary values of the limits and returns them in transaction receipt.
pub struct LimitsModule {
    config: TransactionLimitsConfig,
    number_of_substates_in_track: usize,
    _number_of_substates_in_heap: usize,
}

impl LimitsModule {
    pub fn new(limits_config: TransactionLimitsConfig) -> Self {
        LimitsModule {
            config: limits_config,
            number_of_substates_in_track: 0,
            _number_of_substates_in_heap: 0,
        }
    }

    pub fn config(&self) -> &TransactionLimitsConfig {
        &self.config
    }

    pub fn process_store_access(&mut self, store_access: &StoreAccess) -> Result<(), RuntimeError> {
        match store_access {
            StoreAccess::ReadFromDb(_) | StoreAccess::ReadFromDbNotFound => {}
            StoreAccess::NewEntryInTrack => {
                self.number_of_substates_in_track += 1;
            }
        }

        if self.number_of_substates_in_track > self.config.max_number_of_substates_in_track {
            Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::TooManyEntriesInTrack,
                ),
            ))
        } else {
            Ok(())
        }
    }
}

impl<V: SystemCallbackObject> SystemModule<SystemConfig<V>> for LimitsModule {
    fn before_invoke<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        invocation: &KernelInvocation,
    ) -> Result<(), RuntimeError> {
        // Check depth
        let current_depth = api.kernel_get_current_depth();
        if current_depth == api.kernel_get_system().modules.costing.max_call_depth {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxCallDepthLimitReached,
                ),
            ));
        }

        // Check input size
        let limits = &mut api.kernel_get_system().modules.limits.config;
        let input_size = invocation.len();
        if input_size > limits.max_invoke_payload_size {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxInvokePayloadSizeExceeded(input_size),
                ),
            ));
        }

        Ok(())
    }

    fn on_create_node<Y: KernelInternalApi<SystemConfig<V>>>(
        api: &mut Y,
        event: &CreateNodeEvent,
    ) -> Result<(), RuntimeError> {
        let limits = &mut api.kernel_get_system().modules.limits.config;

        match event {
            CreateNodeEvent::Start(_node_id, node_substates) => {
                for partitions in node_substates.values() {
                    for (_, value) in partitions {
                        if value.len() > limits.max_substate_size {
                            return Err(RuntimeError::SystemModuleError(
                                SystemModuleError::TransactionLimitsError(
                                    TransactionLimitsError::MaxSubstateSizeExceeded(value.len()),
                                ),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn on_write_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        _lock_handle: LockHandle,
        value_size: usize,
    ) -> Result<(), RuntimeError> {
        let limits = &mut api.kernel_get_system().modules.limits.config;

        if value_size > limits.max_substate_size {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxSubstateSizeExceeded(value_size),
                ),
            ));
        }

        Ok(())
    }

    fn on_store_access(
        store_access: &StoreAccess,
        system: &mut SystemConfig<V>,
    ) -> Result<(), RuntimeError> {
        system.modules.limits.process_store_access(store_access)
    }
}
