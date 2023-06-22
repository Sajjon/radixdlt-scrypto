use crate::api::node_modules::metadata::MetadataInit;
use crate::blueprints::resource::*;
use crate::types::*;
use crate::*;
use radix_engine_common::data::manifest::model::ManifestAddressReservation;
use radix_engine_common::data::manifest::model::ManifestBlobRef;
use sbor::rust::collections::BTreeMap;
use sbor::rust::collections::BTreeSet;
use sbor::rust::string::String;
use sbor::rust::vec::Vec;
use scrypto_schema::BlueprintSchemaInit;

pub const PACKAGE_BLUEPRINT: &str = "Package";

pub const PACKAGE_PUBLISH_WASM_IDENT: &str = "publish_wasm";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub struct PackagePublishWasmInput {
    pub code: Vec<u8>,
    pub setup: PackageDefinition,
    pub metadata: MetadataInit,
}

#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct PackagePublishWasmManifestInput {
    pub code: ManifestBlobRef,
    pub setup: PackageDefinition,
    pub metadata: MetadataInit,
}

pub type PackagePublishWasmOutput = (PackageAddress, Bucket);

pub const PACKAGE_PUBLISH_WASM_ADVANCED_IDENT: &str = "publish_wasm_advanced";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct PackagePublishWasmAdvancedInput {
    pub package_address: Option<GlobalAddressReservation>,
    pub code: Vec<u8>,
    pub setup: PackageDefinition,
    pub metadata: MetadataInit,
    pub owner_rule: OwnerRole,
}

#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct PackagePublishWasmAdvancedManifestInput {
    pub package_address: Option<ManifestAddressReservation>,
    pub code: ManifestBlobRef,
    pub setup: PackageDefinition,
    pub metadata: MetadataInit,
    pub owner_rule: OwnerRole,
}

pub type PackagePublishWasmAdvancedOutput = PackageAddress;

pub const PACKAGE_PUBLISH_NATIVE_IDENT: &str = "publish_native";

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor)]
pub struct PackagePublishNativeInput {
    pub package_address: Option<GlobalAddressReservation>,
    pub native_package_code_id: u8,
    pub setup: PackageDefinition,
    pub metadata: MetadataInit,
}

#[derive(Debug, Clone, Eq, PartialEq, ManifestSbor)]
pub struct PackagePublishNativeManifestInput {
    pub package_address: Option<ManifestAddressReservation>,
    pub native_package_code_id: u8,
    pub setup: PackageDefinition,
    pub metadata: MetadataInit,
}

pub type PackagePublishNativeOutput = PackageAddress;

pub const PACKAGE_CLAIM_ROYALTIES_IDENT: &str = "PackageRoyalty_claim_royalties";

#[derive(
    Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestCategorize, ManifestEncode, ManifestDecode,
)]
pub struct PackageClaimRoyaltiesInput {}

pub type PackageClaimRoyaltiesOutput = Bucket;

#[derive(Debug, Clone, Eq, PartialEq, Default, ScryptoSbor, ManifestSbor)]
pub struct PackageDefinition {
    pub blueprints: BTreeMap<String, BlueprintDefinitionInit>,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub enum BlueprintType {
    Outer,
    Inner { outer_blueprint: String },
}

impl Default for BlueprintType {
    fn default() -> Self {
        BlueprintType::Outer
    }
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub struct BlueprintDefinitionInit {
    pub blueprint_type: BlueprintType,
    pub feature_set: BTreeSet<String>,
    pub dependencies: BTreeSet<GlobalAddress>,
    pub schema: BlueprintSchemaInit,
    pub royalty_config: PackageRoyaltyConfig,
    pub auth_config: AuthConfig,
}

impl Default for BlueprintDefinitionInit {
    fn default() -> Self {
        Self {
            blueprint_type: BlueprintType::default(),
            feature_set: BTreeSet::default(),
            dependencies: BTreeSet::default(),
            schema: BlueprintSchemaInit::default(),
            royalty_config: PackageRoyaltyConfig::default(),
            auth_config: AuthConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default, ScryptoSbor, ManifestSbor)]
pub struct AuthConfig {
    pub function_auth: FunctionAuth,
    pub method_auth: MethodAuthTemplate,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub enum FunctionAuth {
    /// All functions are accessible
    AllowAll,
    /// Functions are protected by an access rule
    AccessRules(BTreeMap<String, AccessRule>),
    /// Only the root call frame may call all functions.
    /// Used primarily for transaction processor functions, any other use would
    /// essentially make the function inaccessible for any normal transaction
    RootOnly,
}

impl Default for FunctionAuth {
    fn default() -> Self {
        FunctionAuth::AllowAll
    }
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub enum MethodAuthTemplate {
    /// All methods are accessible
    AllowAll,
    /// Methods are protected by a static method to roles mapping
    StaticRoles(StaticRoles),
}

impl Default for MethodAuthTemplate {
    fn default() -> Self {
        MethodAuthTemplate::StaticRoles(StaticRoles::default())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub enum RoleSpecification {
    /// Roles are specified in the current blueprint and defined in the instantiated object.
    Normal(BTreeMap<RoleKey, RoleList>),
    /// Roles are specified in the *outer* blueprint and defined in the instantiated *outer* object.
    /// This may only be used by inner blueprints and is currently used by the Vault blueprints
    UseOuter,
}

#[derive(Debug, Clone, Eq, PartialEq, ScryptoSbor, ManifestSbor)]
pub struct StaticRoles {
    pub roles: RoleSpecification,
    pub methods: BTreeMap<MethodKey, MethodAccessibility>,
}

impl Default for StaticRoles {
    fn default() -> Self {
        Self {
            methods: BTreeMap::new(),
            roles: RoleSpecification::Normal(BTreeMap::new()),
        }
    }
}
