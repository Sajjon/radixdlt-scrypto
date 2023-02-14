use crate::types::*;
use radix_engine_interface::api::types::*;
use radix_engine_interface::blueprints::access_controller::*;
use radix_engine_interface::blueprints::account::*;
use radix_engine_interface::blueprints::epoch_manager::*;
use radix_engine_interface::blueprints::identity::*;
use radix_engine_interface::blueprints::resource::*;

pub enum CostingEntry {
    /* invoke */
    Invoke { input_size: u32 },

    /* node */
    CreateNode { size: u32 },
    DropNode { size: u32 },

    /* substate */
    LockSubstate,
    ReadSubstate { size: u32 },
    WriteSubstate { size: u32 },
    DropLock,
    // TODO: more costing after API becomes stable.
}

#[derive(Debug, Clone, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct FeeTable {
    tx_base_fee: u32,
    tx_payload_cost_per_byte: u32,
    tx_signature_verification_per_sig: u32,
    tx_blob_price_per_byte: u32,
    fixed_low: u32,
    fixed_medium: u32,
    fixed_high: u32,
    wasm_instantiation_per_byte: u32,
}

impl FeeTable {
    pub fn new() -> Self {
        Self {
            tx_base_fee: 50_000,
            tx_payload_cost_per_byte: 5,
            tx_signature_verification_per_sig: 100_000,
            tx_blob_price_per_byte: 5,
            wasm_instantiation_per_byte: 1, // TODO: Re-enable WASM instantiation cost if it's unavoidable
            fixed_low: 500,
            fixed_medium: 2500,
            fixed_high: 5000,
        }
    }

    pub fn tx_base_fee(&self) -> u32 {
        self.tx_base_fee
    }

    pub fn tx_payload_cost_per_byte(&self) -> u32 {
        self.tx_payload_cost_per_byte
    }

    pub fn tx_signature_verification_per_sig(&self) -> u32 {
        self.tx_signature_verification_per_sig
    }

    pub fn tx_blob_price_per_byte(&self) -> u32 {
        self.tx_blob_price_per_byte
    }

    pub fn wasm_instantiation_per_byte(&self) -> u32 {
        self.wasm_instantiation_per_byte
    }

    pub fn run_cost(&self, identifier: &ScryptoFnIdentifier) -> u32 {
        match (
            identifier.package_address,
            identifier.blueprint_name.as_str(),
        ) {
            (RESOURCE_MANAGER_PACKAGE, RESOURCE_MANAGER_BLUEPRINT) => {
                match identifier.ident.as_str() {
                    RESOURCE_MANAGER_CREATE_FUNGIBLE_IDENT => self.fixed_high,
                    RESOURCE_MANAGER_CREATE_FUNGIBLE_WITH_INITIAL_SUPPLY_IDENT => self.fixed_high,
                    RESOURCE_MANAGER_CREATE_FUNGIBLE_WITH_INITIAL_SUPPLY_AND_ADDRESS_IDENT => {
                        self.fixed_high
                    }
                    RESOURCE_MANAGER_CREATE_NON_FUNGIBLE_IDENT => self.fixed_high,
                    RESOURCE_MANAGER_CREATE_NON_FUNGIBLE_WITH_INITIAL_SUPPLY_IDENT => {
                        self.fixed_high
                    }
                    RESOURCE_MANAGER_CREATE_NON_FUNGIBLE_WITH_ADDRESS_IDENT => self.fixed_high,
                    RESOURCE_MANAGER_CREATE_UUID_NON_FUNGIBLE_WITH_INITIAL_SUPPLY => {
                        self.fixed_high
                    }
                    _ => self.fixed_low,
                }
            }
            (IDENTITY_PACKAGE, IDENTITY_BLUEPRINT) => match identifier.ident.as_str() {
                IDENTITY_CREATE_IDENT => self.fixed_low,
                _ => self.fixed_low,
            },
            (EPOCH_MANAGER_PACKAGE, EPOCH_MANAGER_BLUEPRINT) => match identifier.ident.as_str() {
                EPOCH_MANAGER_CREATE_IDENT => self.fixed_low,
                _ => self.fixed_low,
            },
            (ACCESS_CONTROLLER_PACKAGE, ACCESS_CONTROLLER_BLUEPRINT) => {
                match identifier.ident.as_str() {
                    ACCESS_CONTROLLER_CREATE_GLOBAL_IDENT => self.fixed_low,
                    _ => self.fixed_low,
                }
            }
            (ACCOUNT_PACKAGE, ACCOUNT_BLUEPRINT) => match identifier.ident.as_str() {
                ACCOUNT_CREATE_LOCAL_IDENT => self.fixed_low,
                ACCOUNT_CREATE_GLOBAL_IDENT => self.fixed_low,
                _ => self.fixed_low,
            },

            _ => 0u32,
        }
    }

    pub fn run_native_fn_cost(&self, native_fn: &NativeFn) -> u32 {
        match native_fn {
            NativeFn::AuthZoneStack(auth_zone_ident) => {
                match auth_zone_ident {
                    AuthZoneStackFn::Pop => self.fixed_low,
                    AuthZoneStackFn::Push => self.fixed_low,
                    AuthZoneStackFn::CreateProof => self.fixed_high, // TODO: charge differently based on auth zone size and fungibility
                    AuthZoneStackFn::CreateProofByAmount => self.fixed_high,
                    AuthZoneStackFn::CreateProofByIds => self.fixed_high,
                    AuthZoneStackFn::Clear => self.fixed_high,
                    AuthZoneStackFn::Drain => self.fixed_high,
                    AuthZoneStackFn::AssertAccessRule => self.fixed_high,
                }
            }
            NativeFn::EpochManager(epoch_manager_method) => match epoch_manager_method {
                EpochManagerFn::GetCurrentEpoch => self.fixed_low,
                EpochManagerFn::NextRound => self.fixed_low,
                EpochManagerFn::SetEpoch => self.fixed_low,
                EpochManagerFn::UpdateValidator => self.fixed_low,
                EpochManagerFn::CreateValidator => self.fixed_low,
            },
            NativeFn::Validator(validator_fn) => match validator_fn {
                ValidatorFn::Register => self.fixed_low,
                ValidatorFn::Unregister => self.fixed_low,
                ValidatorFn::Stake => self.fixed_low,
                ValidatorFn::Unstake => self.fixed_low,
                ValidatorFn::ClaimXrd => self.fixed_low,
                ValidatorFn::UpdateKey => self.fixed_low,
                ValidatorFn::UpdateAcceptDelegatedStake => self.fixed_low,
            },
            NativeFn::Clock(clock_method) => match clock_method {
                ClockFn::SetCurrentTime => self.fixed_low,
                ClockFn::GetCurrentTime => self.fixed_high,
                ClockFn::CompareCurrentTime => self.fixed_high,
            },
            NativeFn::Bucket(bucket_ident) => match bucket_ident {
                BucketFn::Take => self.fixed_medium,
                BucketFn::TakeNonFungibles => self.fixed_medium,
                BucketFn::GetNonFungibleLocalIds => self.fixed_medium,
                BucketFn::Put => self.fixed_medium,
                BucketFn::GetAmount => self.fixed_low,
                BucketFn::GetResourceAddress => self.fixed_low,
                BucketFn::CreateProof => self.fixed_low,
            },
            NativeFn::Proof(proof_ident) => match proof_ident {
                ProofFn::GetAmount => self.fixed_low,
                ProofFn::GetNonFungibleLocalIds => self.fixed_low,
                ProofFn::GetResourceAddress => self.fixed_low,
                ProofFn::Clone => self.fixed_low,
            },
            NativeFn::ResourceManager(resource_manager_ident) => match resource_manager_ident {
                ResourceManagerFn::BurnBucket => self.fixed_low,
                ResourceManagerFn::UpdateVaultAuth => self.fixed_medium,
                ResourceManagerFn::SetVaultAuthMutability => self.fixed_medium,
                ResourceManagerFn::CreateVault => self.fixed_medium,
                ResourceManagerFn::CreateBucket => self.fixed_medium,
                ResourceManagerFn::MintNonFungible => self.fixed_high,
                ResourceManagerFn::MintUuidNonFungible => self.fixed_high,
                ResourceManagerFn::MintFungible => self.fixed_high,
                ResourceManagerFn::GetResourceType => self.fixed_low,
                ResourceManagerFn::GetTotalSupply => self.fixed_low,
                ResourceManagerFn::UpdateNonFungibleData => self.fixed_medium,
                ResourceManagerFn::NonFungibleExists => self.fixed_low,
                ResourceManagerFn::GetNonFungible => self.fixed_medium,
                ResourceManagerFn::Burn => self.fixed_medium,
            },
            NativeFn::Worktop(worktop_ident) => match worktop_ident {
                WorktopFn::Put => self.fixed_medium,
                WorktopFn::TakeAmount => self.fixed_medium,
                WorktopFn::TakeAll => self.fixed_medium,
                WorktopFn::TakeNonFungibles => self.fixed_medium,
                WorktopFn::AssertContains => self.fixed_low,
                WorktopFn::AssertContainsAmount => self.fixed_low,
                WorktopFn::AssertContainsNonFungibles => self.fixed_low,
                WorktopFn::Drain => self.fixed_low,
            },
            NativeFn::Logger(logger_method) => match logger_method {
                LoggerFn::Log => self.fixed_low,
            },
            NativeFn::AccessRulesChain(component_ident) => match component_ident {
                AccessRulesChainFn::AddAccessCheck => self.fixed_low,
                AccessRulesChainFn::SetMethodAccessRule => self.fixed_low,
                AccessRulesChainFn::SetMethodMutability => self.fixed_low,
                AccessRulesChainFn::SetGroupAccessRule => self.fixed_low,
                AccessRulesChainFn::SetGroupMutability => self.fixed_low,
                AccessRulesChainFn::GetLength => self.fixed_low,
            },
            NativeFn::Metadata(metadata_method) => match metadata_method {
                MetadataFn::Set => self.fixed_low,
                MetadataFn::Get => self.fixed_low,
            },
            NativeFn::Component(method_ident) => match method_ident {
                ComponentFn::Globalize => self.fixed_high,
                ComponentFn::GlobalizeWithOwner => self.fixed_high,
                ComponentFn::SetRoyaltyConfig => self.fixed_medium,
                ComponentFn::ClaimRoyalty => self.fixed_medium,
            },
            NativeFn::Package(method_ident) => match method_ident {
                PackageFn::Publish => self.fixed_high,
                PackageFn::PublishNative => self.fixed_high,
                PackageFn::SetRoyaltyConfig => self.fixed_medium,
                PackageFn::ClaimRoyalty => self.fixed_medium,
            },
            NativeFn::Vault(vault_ident) => {
                match vault_ident {
                    VaultFn::Put => self.fixed_medium,
                    VaultFn::Take => self.fixed_medium, // TODO: revisit this if vault is not loaded in full
                    VaultFn::TakeNonFungibles => self.fixed_medium,
                    VaultFn::GetAmount => self.fixed_low,
                    VaultFn::GetResourceAddress => self.fixed_low,
                    VaultFn::GetNonFungibleLocalIds => self.fixed_medium,
                    VaultFn::CreateProof => self.fixed_high,
                    VaultFn::CreateProofByAmount => self.fixed_high,
                    VaultFn::CreateProofByIds => self.fixed_high,
                    VaultFn::LockFee => self.fixed_medium,
                    VaultFn::Recall => self.fixed_low,
                    VaultFn::RecallNonFungibles => self.fixed_low,
                }
            }
            NativeFn::TransactionRuntime(ident) => match ident {
                TransactionRuntimeFn::GetHash => self.fixed_low,
                TransactionRuntimeFn::GenerateUuid => self.fixed_low,
            },
            NativeFn::TransactionProcessor(transaction_processor_fn) => {
                match transaction_processor_fn {
                    TransactionProcessorFn::Run => self.fixed_high,
                }
            }
            // TODO: Investigate what sensible costing for native components looks like
            NativeFn::Account(account_fn) => match account_fn {
                AccountFn::LockFee => self.fixed_low,
                AccountFn::LockContingentFee => self.fixed_low,

                AccountFn::Deposit => self.fixed_low,
                AccountFn::DepositBatch => self.fixed_low,

                AccountFn::WithdrawAll => self.fixed_low,
                AccountFn::Withdraw => self.fixed_low,
                AccountFn::WithdrawNonFungibles => self.fixed_low,

                AccountFn::LockFeeAndWithdrawAll => self.fixed_low,
                AccountFn::LockFeeAndWithdraw => self.fixed_low,
                AccountFn::LockFeeAndWithdrawNonFungibles => self.fixed_low,

                AccountFn::CreateProof => self.fixed_low,
                AccountFn::CreateProofByAmount => self.fixed_low,
                AccountFn::CreateProofByIds => self.fixed_low,
            },
            NativeFn::AccessController(access_controller_fn) => match access_controller_fn {
                AccessControllerFn::CreateProof => self.fixed_low,

                AccessControllerFn::InitiateRecoveryAsPrimary => self.fixed_low,
                AccessControllerFn::InitiateRecoveryAsRecovery => self.fixed_low,

                AccessControllerFn::QuickConfirmPrimaryRoleRecoveryProposal => self.fixed_low,
                AccessControllerFn::QuickConfirmRecoveryRoleRecoveryProposal => self.fixed_low,

                AccessControllerFn::TimedConfirmRecovery => self.fixed_low,

                AccessControllerFn::CancelPrimaryRoleRecoveryProposal => self.fixed_low,
                AccessControllerFn::CancelRecoveryRoleRecoveryProposal => self.fixed_low,

                AccessControllerFn::LockPrimaryRole => self.fixed_low,
                AccessControllerFn::UnlockPrimaryRole => self.fixed_low,

                AccessControllerFn::StopTimedRecovery => self.fixed_low,
            },
            NativeFn::Root => 0,
        }
    }

    pub fn kernel_api_cost(&self, entry: CostingEntry) -> u32 {
        match entry {
            CostingEntry::Invoke { input_size } => self.fixed_low + (10 * input_size) as u32,

            CostingEntry::CreateNode { size } => self.fixed_medium + (100 * size) as u32,
            CostingEntry::DropNode { size } => self.fixed_medium + (100 * size) as u32,

            CostingEntry::LockSubstate => self.fixed_high,
            CostingEntry::ReadSubstate { size } => self.fixed_medium + 100 * size,
            CostingEntry::WriteSubstate { size } => self.fixed_medium + 1000 * size,
            CostingEntry::DropLock => self.fixed_high,
        }
    }
}