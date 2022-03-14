use sbor::*;
use scrypto::engine::types::*;
use scrypto::rust::collections::BTreeSet;
use scrypto::rust::string::String;
use scrypto::rust::vec::Vec;

use crate::model::ValidatedData;

/// Represents an unvalidated transaction.
#[derive(Debug, Clone, TypeId, Encode, Decode, PartialEq, Eq)]
pub struct Transaction {
    pub instructions: Vec<Instruction>,
}

/// Represents an unvalidated instruction in transaction
#[derive(Debug, Clone, TypeId, Encode, Decode, PartialEq, Eq)]
pub enum Instruction {
    /// Takes fixed amount resource from worktop.
    TakeFromWorktop {
        amount: Decimal,
        resource_def_id: ResourceDefId,
    },

    /// Takes all of a given resource from worktop.
    TakeAllFromWorktop { resource_def_id: ResourceDefId },

    /// Takes non-fungibles from worktop.
    TakeNonFungiblesFromWorktop {
        keys: BTreeSet<NonFungibleId>,
        resource_def_id: ResourceDefId,
    },

    /// Returns resource to worktop.
    ReturnToWorktop { bucket_id: BucketId },

    /// Asserts worktop contains at least this amount.
    AssertWorktopContains {
        amount: Decimal,
        resource_def_id: ResourceDefId,
    },

    /// Takes from the auth worktop.
    TakeFromAuthWorktop { index: u32 },

    /// Put a proof on the auth worktop.
    PutOnAuthWorktop { proof_id: ProofId },

    /// Creates a proof.
    CreateBucketProof { bucket_id: BucketId },

    /// Clones a proof.
    CloneProof { proof_id: ProofId },

    /// Drops a proof.
    DropProof { proof_id: ProofId },

    /// Calls a blueprint function.
    ///
    /// Buckets and proofs in arguments moves from transaction context to the callee.
    CallFunction {
        package_id: PackageId,
        blueprint_name: String,
        function: String,
        args: Vec<Vec<u8>>,
    },

    /// Calls a component method.
    ///
    /// Buckets and proofs in arguments moves from transaction context to the callee.
    CallMethod {
        component_id: ComponentId,
        method: String,
        args: Vec<Vec<u8>>,
    },

    /// With method with all resources from transaction context.
    CallMethodWithAllResources {
        component_id: ComponentId,
        method: String,
    },

    /// Publishes a package.
    PublishPackage { code: Vec<u8> },

    /// Marks the end of transaction with signatures.
    /// TODO: replace public key with signature.
    End { signatures: Vec<EcdsaPublicKey> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTransaction {
    pub instructions: Vec<ValidatedInstruction>,
    pub signers: Vec<EcdsaPublicKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedInstruction {
    TakeFromWorktop {
        amount: Decimal,
        resource_def_id: ResourceDefId,
    },
    TakeAllFromWorktop {
        resource_def_id: ResourceDefId,
    },
    TakeNonFungiblesFromWorktop {
        keys: BTreeSet<NonFungibleId>,
        resource_def_id: ResourceDefId,
    },
    ReturnToWorktop {
        bucket_id: BucketId,
    },
    AssertWorktopContains {
        amount: Decimal,
        resource_def_id: ResourceDefId,
    },
    TakeFromAuthWorktop {
        index: u32,
    },
    PutOnAuthWorktop {
        proof_id: ProofId,
    },
    CreateBucketProof {
        bucket_id: BucketId,
    },
    CloneProof {
        proof_id: ProofId,
    },
    DropProof {
        proof_id: ProofId,
    },
    CallFunction {
        package_id: PackageId,
        blueprint_name: String,
        function: String,
        args: Vec<ValidatedData>,
    },
    CallMethod {
        component_id: ComponentId,
        method: String,
        args: Vec<ValidatedData>,
    },
    CallMethodWithAllResources {
        component_id: ComponentId,
        method: String,
    },
    PublishPackage {
        code: Vec<u8>,
    },
}