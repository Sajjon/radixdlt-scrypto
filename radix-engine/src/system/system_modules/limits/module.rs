use crate::kernel::kernel_api::{KernelInternalApi, KernelInvocation};
use crate::kernel::kernel_callback_api::{
    CreateNodeEvent, DrainSubstatesEvent, DropNodeEvent, MoveModuleEvent, OpenSubstateEvent,
    ReadSubstateEvent, RemoveSubstateEvent, ScanKeysEvent, ScanSortedSubstatesEvent,
    SetSubstateEvent, WriteSubstateEvent,
};
use crate::system::actor::Actor;
use crate::system::module::SystemModule;
use crate::system::system_callback::SystemConfig;
use crate::system::system_callback_api::SystemCallbackObject;
use crate::track::interface::IOAccess;
use crate::types::*;
use crate::{errors::RuntimeError, errors::SystemModuleError, kernel::kernel_api::KernelApi};

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum TransactionLimitsError {
    MaxSubstateKeySizeExceeded(usize),
    MaxSubstateSizeExceeded(usize),
    MaxInvokePayloadSizeExceeded(usize),
    MaxCallDepthLimitReached,
    TrackSubstateSizeExceeded { actual: usize, max: usize },
    HeapSubstateSizeExceeded { actual: usize, max: usize },
    LogSizeTooLarge { actual: usize, max: usize },
    EventSizeTooLarge { actual: usize, max: usize },
    PanicMessageSizeTooLarge { actual: usize, max: usize },
    TooManyLogs,
    TooManyEvents,
}

pub struct TransactionLimitsConfig {
    pub max_heap_substate_total_bytes: usize,
    pub max_track_substate_total_bytes: usize,
    pub max_substate_key_size: usize,
    pub max_substate_value_size: usize,
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
    heap_substate_total_bytes: usize,
    track_substate_total_bytes: usize,
}

impl LimitsModule {
    pub fn new(limits_config: TransactionLimitsConfig) -> Self {
        LimitsModule {
            config: limits_config,
            heap_substate_total_bytes: 0,
            track_substate_total_bytes: 0,
        }
    }

    pub fn config(&self) -> &TransactionLimitsConfig {
        &self.config
    }

    pub fn process_substate_key(&self, substate_key: &SubstateKey) -> Result<(), RuntimeError> {
        let len = match substate_key {
            SubstateKey::Map(map_key) => map_key.len(),
            SubstateKey::Sorted((_sort_key, map_key)) => map_key.len() + 2,
            SubstateKey::Field(_field_key) => 1,
        };

        if len > self.config.max_substate_key_size {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxSubstateKeySizeExceeded(len),
                ),
            ));
        }

        Ok(())
    }

    pub fn process_substate_value(&self, value: &IndexedScryptoValue) -> Result<(), RuntimeError> {
        if value.len() > self.config.max_substate_value_size {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::MaxSubstateSizeExceeded(value.len()),
                ),
            ));
        }

        Ok(())
    }

    pub fn process_io_access(&mut self, io_access: &IOAccess) -> Result<(), RuntimeError> {
        match io_access {
            IOAccess::ReadFromDb(..) | IOAccess::ReadFromDbNotFound(..) => {}

            IOAccess::HeapSubstateUpdated {
                canonical_substate_key,
                old_size,
                new_size,
            } => {
                if old_size.is_none() {
                    self.heap_substate_total_bytes += canonical_substate_key.len();
                }
                if new_size.is_none() {
                    self.heap_substate_total_bytes -= canonical_substate_key.len();
                }

                self.heap_substate_total_bytes += new_size.unwrap_or_default();
                self.heap_substate_total_bytes -= old_size.unwrap_or_default();
            }
            IOAccess::TrackSubstateUpdated {
                canonical_substate_key,
                old_size,
                new_size,
            } => {
                if old_size.is_none() {
                    self.track_substate_total_bytes += canonical_substate_key.len();
                }
                if new_size.is_none() {
                    self.track_substate_total_bytes -= canonical_substate_key.len();
                }

                self.track_substate_total_bytes += new_size.unwrap_or_default();
                self.track_substate_total_bytes -= old_size.unwrap_or_default();
            }
        }

        if self.heap_substate_total_bytes > self.config.max_heap_substate_total_bytes {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::HeapSubstateSizeExceeded {
                        actual: self.heap_substate_total_bytes,
                        max: self.config.max_heap_substate_total_bytes,
                    },
                ),
            ));
        }

        if self.track_substate_total_bytes > self.config.max_track_substate_total_bytes {
            return Err(RuntimeError::SystemModuleError(
                SystemModuleError::TransactionLimitsError(
                    TransactionLimitsError::TrackSubstateSizeExceeded {
                        actual: self.track_substate_total_bytes,
                        max: self.config.max_track_substate_total_bytes,
                    },
                ),
            ));
        }

        Ok(())
    }
}

impl<V: SystemCallbackObject> SystemModule<SystemConfig<V>> for LimitsModule {
    fn before_invoke<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        invocation: &KernelInvocation<Actor>,
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
        let limits = &mut api.kernel_get_system().modules.limits;

        match event {
            CreateNodeEvent::Start(_node_id, node_substates) => {
                for partitions in node_substates.values() {
                    for (key, value) in partitions {
                        limits.process_substate_key(key)?;
                        limits.process_substate_value(value)?;
                    }
                }
            }
            CreateNodeEvent::IOAccess(io_access) => {
                limits.process_io_access(io_access)?;
            }
            CreateNodeEvent::End(..) => {}
        }

        Ok(())
    }

    fn on_drop_node<Y: KernelInternalApi<SystemConfig<V>>>(
        api: &mut Y,
        event: &DropNodeEvent,
    ) -> Result<(), RuntimeError> {
        let limits = &mut api.kernel_get_system().modules.limits;

        match event {
            DropNodeEvent::IOAccess(io_access) => {
                limits.process_io_access(io_access)?;
            }
            DropNodeEvent::Start(..) | DropNodeEvent::End(..) => {}
        }

        Ok(())
    }

    fn on_move_module<Y: KernelInternalApi<SystemConfig<V>>>(
        api: &mut Y,
        event: &MoveModuleEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            MoveModuleEvent::IOAccess(io_access) => {
                api.kernel_get_system()
                    .modules
                    .limits
                    .process_io_access(io_access)?;
            }
        }

        Ok(())
    }

    fn on_open_substate<Y: KernelInternalApi<SystemConfig<V>>>(
        api: &mut Y,
        event: &OpenSubstateEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            OpenSubstateEvent::Start { substate_key, .. } => {
                api.kernel_get_system()
                    .modules
                    .limits
                    .process_substate_key(substate_key)?;
            }
            OpenSubstateEvent::IOAccess(io_access) => {
                api.kernel_get_system()
                    .modules
                    .limits
                    .process_io_access(io_access)?;
            }
            OpenSubstateEvent::End { .. } => {}
        }

        Ok(())
    }

    fn on_read_substate<Y: KernelInternalApi<SystemConfig<V>>>(
        api: &mut Y,
        event: &ReadSubstateEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            ReadSubstateEvent::IOAccess(io_access) => {
                api.kernel_get_system()
                    .modules
                    .limits
                    .process_io_access(io_access)?;
            }
            ReadSubstateEvent::OnRead { .. } => {}
        }

        Ok(())
    }

    fn on_write_substate<Y: KernelInternalApi<SystemConfig<V>>>(
        api: &mut Y,
        event: &WriteSubstateEvent,
    ) -> Result<(), RuntimeError> {
        let limits = &mut api.kernel_get_system().modules.limits;

        match event {
            WriteSubstateEvent::Start { value, .. } => {
                limits.process_substate_value(value)?;
            }
            WriteSubstateEvent::IOAccess(io_access) => {
                api.kernel_get_system()
                    .modules
                    .limits
                    .process_io_access(io_access)?;
            }
        }

        Ok(())
    }

    fn on_set_substate(
        system: &mut SystemConfig<V>,
        event: &SetSubstateEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            SetSubstateEvent::Start(_node_id, _partition_num, substate_key, substate_value) => {
                system.modules.limits.process_substate_key(substate_key)?;
                system
                    .modules
                    .limits
                    .process_substate_value(substate_value)?;
            }
            SetSubstateEvent::IOAccess(io_access) => {
                system.modules.limits.process_io_access(io_access)?;
            }
        }

        Ok(())
    }

    fn on_remove_substate(
        system: &mut SystemConfig<V>,
        event: &RemoveSubstateEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            RemoveSubstateEvent::Start(_node_id, _partition_num, substate_key) => {
                system.modules.limits.process_substate_key(substate_key)?;
            }
            RemoveSubstateEvent::IOAccess(io_access) => {
                system.modules.limits.process_io_access(io_access)?;
            }
        }

        Ok(())
    }

    fn on_scan_keys(
        system: &mut SystemConfig<V>,
        event: &ScanKeysEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            ScanKeysEvent::IOAccess(io_access) => {
                system.modules.limits.process_io_access(io_access)?;
            }
            ScanKeysEvent::Start => {}
        }

        Ok(())
    }

    fn on_drain_substates(
        system: &mut SystemConfig<V>,
        event: &DrainSubstatesEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            DrainSubstatesEvent::IOAccess(io_access) => {
                system.modules.limits.process_io_access(io_access)?;
            }
            DrainSubstatesEvent::Start(_) => {}
        }

        Ok(())
    }

    fn on_scan_sorted_substates(
        system: &mut SystemConfig<V>,
        event: &ScanSortedSubstatesEvent,
    ) -> Result<(), RuntimeError> {
        match event {
            ScanSortedSubstatesEvent::IOAccess(io_access) => {
                system.modules.limits.process_io_access(io_access)?;
            }
            ScanSortedSubstatesEvent::Start => {}
        }

        Ok(())
    }
}
