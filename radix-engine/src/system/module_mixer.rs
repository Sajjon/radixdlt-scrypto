use crate::errors::*;
use crate::kernel::actor::Actor;
use crate::kernel::call_frame::Message;
use crate::kernel::kernel_api::KernelApi;
use crate::kernel::kernel_api::KernelInvocation;
use crate::system::module::SystemModule;
use crate::system::system_callback::SystemConfig;
use crate::system::system_callback_api::SystemCallbackObject;
use crate::system::system_modules::auth::AuthModule;
use crate::system::system_modules::costing::CostingModule;
use crate::system::system_modules::costing::FeeTable;
use crate::system::system_modules::costing::SystemLoanFeeReserve;
use crate::system::system_modules::execution_trace::ExecutionTraceModule;
use crate::system::system_modules::kernel_trace::KernelTraceModule;
use crate::system::system_modules::limits::{LimitsModule, TransactionLimitsConfig};
use crate::system::system_modules::node_move::NodeMoveModule;
use crate::system::system_modules::transaction_events::TransactionEventsModule;
use crate::system::system_modules::transaction_runtime::TransactionRuntimeModule;
use crate::track::interface::{NodeSubstates, StoreAccessInfo};
use crate::transaction::ExecutionConfig;
use crate::types::*;
use bitflags::bitflags;
use paste::paste;
use radix_engine_interface::api::field_lock_api::LockFlags;
use radix_engine_interface::crypto::Hash;
use resources_tracker_macro::trace_resources;
use transaction::model::AuthZoneParams;

bitflags! {
    pub struct EnabledModules: u32 {
        // Kernel trace, for debugging only
        const KERNEL_TRACE = 0x1 << 0;

        // Limits, costing and auth
        const LIMITS = 0x01 << 1;
        const COSTING = 0x01 << 2;
        const AUTH = 0x01 << 3;
        const NODE_MOVE = 0x01 << 4;

        // Transaction runtime data
        const TRANSACTION_RUNTIME = 0x01 << 5;
        const TRANSACTION_EVENTS = 0x01 << 6;

        // Execution trace, for preview only
        const EXECUTION_TRACE = 0x01 << 7;
    }
}

impl EnabledModules {
    /// The difference between genesis transaction and system transaction is "no auth".
    /// TODO: double check if this is the right assumption.
    pub fn for_genesis_transaction() -> Self {
        Self::LIMITS | Self::NODE_MOVE | Self::TRANSACTION_RUNTIME | Self::TRANSACTION_EVENTS
    }

    pub fn for_system_transaction() -> Self {
        Self::LIMITS
            | Self::AUTH
            | Self::NODE_MOVE
            | Self::TRANSACTION_RUNTIME
            | Self::TRANSACTION_EVENTS
    }

    pub fn for_notarized_transaction() -> Self {
        Self::LIMITS
            | Self::COSTING
            | Self::AUTH
            | Self::NODE_MOVE
            | Self::TRANSACTION_RUNTIME
            | Self::TRANSACTION_EVENTS
    }

    pub fn for_test_transaction() -> Self {
        Self::for_notarized_transaction() | Self::KERNEL_TRACE
    }

    pub fn for_preview() -> Self {
        Self::for_notarized_transaction() | Self::EXECUTION_TRACE
    }
}

#[allow(dead_code)]
pub struct SystemModuleMixer {
    // TODO: Use option instead of default for module states?
    // The original reason for performance, but we should double check.

    /* flags */
    enabled_modules: EnabledModules,

    /* states */
    kernel_trace: KernelTraceModule,
    limits: LimitsModule,
    costing: CostingModule,
    auth: AuthModule,
    node_move: NodeMoveModule,
    transaction_runtime: TransactionRuntimeModule,
    transaction_events: TransactionEventsModule,
    execution_trace: ExecutionTraceModule,
}

// Macro generates default modules dispatches call based on passed function name and arguments.
macro_rules! internal_call_dispatch {
    ($api:ident, $fn:ident ( $($param:ident),*) ) => {
        paste! {
        {
            let modules: EnabledModules = $api.kernel_get_system().modules.enabled_modules;
            if modules.contains(EnabledModules::KERNEL_TRACE) {
                KernelTraceModule::[< $fn >]($($param, )*)?;
            }
            if modules.contains(EnabledModules::LIMITS) {
                 LimitsModule::[< $fn >]($($param, )*)?;
            }
            if modules.contains(EnabledModules::COSTING) {
                CostingModule::[< $fn >]($($param, )*)?;
            }
            if modules.contains(EnabledModules::AUTH) {
                AuthModule::[< $fn >]($($param, )*)?;
            }
            if modules.contains(EnabledModules::NODE_MOVE) {
                NodeMoveModule::[< $fn >]($($param, )*)?;
            }
            if modules.contains(EnabledModules::TRANSACTION_RUNTIME) {
                TransactionRuntimeModule::[< $fn >]($($param, )*)?;
            }
            if modules.contains(EnabledModules::TRANSACTION_EVENTS) {
                TransactionEventsModule::[< $fn >]($($param, )*)?;
            }
            if modules.contains(EnabledModules::EXECUTION_TRACE) {
                ExecutionTraceModule::[< $fn >]($($param, )*)?;
            }
            Ok(())
        }
    }};
}

impl SystemModuleMixer {
    pub fn new(
        enabled_modules: EnabledModules,
        tx_hash: Hash,
        auth_zone_params: AuthZoneParams,
        fee_reserve: SystemLoanFeeReserve,
        fee_table: FeeTable,
        payload_len: usize,
        num_of_signatures: usize,
        execution_config: &ExecutionConfig,
    ) -> Self {
        Self {
            enabled_modules,
            kernel_trace: KernelTraceModule {},
            costing: CostingModule {
                fee_reserve,
                fee_table,
                max_call_depth: execution_config.max_call_depth,
                payload_len,
                num_of_signatures,
            },
            node_move: NodeMoveModule {},
            auth: AuthModule {
                params: auth_zone_params.clone(),
                auth_zone_stack: Vec::new(),
            },
            limits: LimitsModule::new(TransactionLimitsConfig {
                max_wasm_memory: execution_config.max_wasm_mem_per_transaction,
                max_wasm_memory_per_call_frame: execution_config.max_wasm_mem_per_call_frame,
                max_substate_read_count: execution_config.max_substate_reads_per_transaction,
                max_substate_write_count: execution_config.max_substate_writes_per_transaction,
                max_substate_size: execution_config.max_substate_size,
                max_invoke_payload_size: execution_config.max_invoke_input_size,
            }),
            execution_trace: ExecutionTraceModule::new(execution_config.max_execution_trace_depth),
            transaction_runtime: TransactionRuntimeModule {
                tx_hash,
                next_id: 0,
                logs: Vec::new(),
            },
            transaction_events: TransactionEventsModule::default(),
        }
    }

    pub fn limits_module(&mut self) -> Option<&mut LimitsModule> {
        if self.enabled_modules.contains(EnabledModules::LIMITS) {
            Some(&mut self.limits)
        } else {
            None
        }
    }

    pub fn costing_module(&mut self) -> Option<&mut CostingModule> {
        if self.enabled_modules.contains(EnabledModules::COSTING) {
            Some(&mut self.costing)
        } else {
            None
        }
    }

    pub fn auth_module(&mut self) -> Option<&mut AuthModule> {
        if self.enabled_modules.contains(EnabledModules::AUTH) {
            Some(&mut self.auth)
        } else {
            None
        }
    }

    pub fn execution_trace_module(&mut self) -> Option<&mut ExecutionTraceModule> {
        if self
            .enabled_modules
            .contains(EnabledModules::EXECUTION_TRACE)
        {
            Some(&mut self.execution_trace)
        } else {
            None
        }
    }

    pub fn transaction_runtime_module(&mut self) -> Option<&mut TransactionRuntimeModule> {
        if self
            .enabled_modules
            .contains(EnabledModules::TRANSACTION_RUNTIME)
        {
            Some(&mut self.transaction_runtime)
        } else {
            None
        }
    }
    pub fn transaction_events_module(&mut self) -> Option<&mut TransactionEventsModule> {
        if self
            .enabled_modules
            .contains(EnabledModules::TRANSACTION_EVENTS)
        {
            Some(&mut self.transaction_events)
        } else {
            None
        }
    }

    pub fn unpack(
        self,
    ) -> (
        LimitsModule,
        CostingModule,
        TransactionRuntimeModule,
        TransactionEventsModule,
        ExecutionTraceModule,
    ) {
        (
            self.limits,
            self.costing,
            self.transaction_runtime,
            self.transaction_events,
            self.execution_trace,
        )
    }
}

//====================================================================
// NOTE: Modules are applied in the reverse order of initialization!
// This has an impact if there is module dependency.
//====================================================================

impl<V: SystemCallbackObject> SystemModule<SystemConfig<V>> for SystemModuleMixer {
    #[trace_resources]
    fn on_init<Y: KernelApi<SystemConfig<V>>>(api: &mut Y) -> Result<(), RuntimeError> {
        let modules: EnabledModules = api.kernel_get_system().modules.enabled_modules;

        // Enable execution trace
        if modules.contains(EnabledModules::EXECUTION_TRACE) {
            ExecutionTraceModule::on_init(api)?;
        }

        // Enable events
        if modules.contains(EnabledModules::TRANSACTION_EVENTS) {
            TransactionEventsModule::on_init(api)?;
        }

        // Enable transaction runtime
        if modules.contains(EnabledModules::TRANSACTION_RUNTIME) {
            TransactionRuntimeModule::on_init(api)?;
        }

        // Enable node move
        if modules.contains(EnabledModules::NODE_MOVE) {
            NodeMoveModule::on_init(api)?;
        }

        // Enable auth
        if modules.contains(EnabledModules::AUTH) {
            AuthModule::on_init(api)?;
        }

        // Enable costing
        if modules.contains(EnabledModules::COSTING) {
            CostingModule::on_init(api)?;
        }

        // Enable transaction limits
        if modules.contains(EnabledModules::LIMITS) {
            LimitsModule::on_init(api)?;
        }

        // Enable kernel trace
        if modules.contains(EnabledModules::KERNEL_TRACE) {
            KernelTraceModule::on_init(api)?;
        }

        Ok(())
    }

    #[trace_resources]
    fn on_teardown<Y: KernelApi<SystemConfig<V>>>(api: &mut Y) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_teardown(api))
    }

    #[trace_resources(log=invocation.len())]
    fn before_invoke<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        invocation: &KernelInvocation,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, before_invoke(api, invocation))
    }

    #[trace_resources]
    fn before_push_frame<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        callee: &Actor,
        update: &mut Message,
        args: &IndexedScryptoValue,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, before_push_frame(api, callee, update, args))
    }

    #[trace_resources]
    fn on_execution_start<Y: KernelApi<SystemConfig<V>>>(api: &mut Y) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_execution_start(api))
    }

    #[trace_resources]
    fn on_execution_finish<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        update: &Message,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_execution_finish(api, update))
    }

    #[trace_resources]
    fn after_pop_frame<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        dropped_actor: &Actor,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, after_pop_frame(api, dropped_actor))
    }

    #[trace_resources(log=output_size)]
    fn after_invoke<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        output_size: usize,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, after_invoke(api, output_size))
    }

    #[trace_resources(log=entity_type)]
    fn on_allocate_node_id<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        entity_type: EntityType,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_allocate_node_id(api, entity_type))
    }

    #[trace_resources]
    fn before_create_node<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        node_id: &NodeId,
        node_substates: &NodeSubstates,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, before_create_node(api, node_id, node_substates))
    }

    #[trace_resources]
    fn after_create_node<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        node_id: &NodeId,
        store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, after_create_node(api, node_id, store_access))
    }

    #[trace_resources]
    fn before_drop_node<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        node_id: &NodeId,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, before_drop_node(api, node_id))
    }

    #[trace_resources]
    fn after_drop_node<Y: KernelApi<SystemConfig<V>>>(api: &mut Y) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, after_drop_node(api))
    }

    #[trace_resources]
    fn before_lock_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        node_id: &NodeId,
        partition_number: &PartitionNumber,
        substate_key: &SubstateKey,
        flags: &LockFlags,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(
            api,
            before_lock_substate(api, node_id, partition_number, substate_key, flags)
        )
    }

    #[trace_resources(log=size)]
    fn after_lock_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        handle: LockHandle,
        store_access: &StoreAccessInfo,
        size: usize,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, after_lock_substate(api, handle, store_access, size))
    }

    #[trace_resources(log=value_size)]
    fn on_read_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        lock_handle: LockHandle,
        value_size: usize,
        store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(
            api,
            on_read_substate(api, lock_handle, value_size, store_access)
        )
    }

    #[trace_resources(log=value_size)]
    fn on_write_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        lock_handle: LockHandle,
        value_size: usize,
        store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(
            api,
            on_write_substate(api, lock_handle, value_size, store_access)
        )
    }

    #[trace_resources]
    fn on_drop_lock<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        lock_handle: LockHandle,
        store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_drop_lock(api, lock_handle, store_access))
    }

    #[trace_resources]
    fn on_scan_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_scan_substate(api, store_access))
    }

    #[trace_resources]
    fn on_set_substate<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_set_substate(api, store_access))
    }

    #[trace_resources]
    fn on_take_substates<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        store_access: &StoreAccessInfo,
    ) -> Result<(), RuntimeError> {
        internal_call_dispatch!(api, on_take_substates(api, store_access))
    }
}
