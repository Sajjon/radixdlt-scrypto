use radix_engine_interface::{
    api::node_modules::auth::AuthAddresses, blueprints::transaction_processor::RuntimeValidation,
};

use crate::internal_prelude::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValidatedNotarizedTransactionV1 {
    pub prepared: PreparedNotarizedTransactionV1,
    pub encoded_instructions: Vec<u8>,
    pub signer_keys: Vec<PublicKey>,
}

impl HasIntentHash for ValidatedNotarizedTransactionV1 {
    fn intent_hash(&self) -> IntentHash {
        self.prepared.intent_hash()
    }
}

impl HasSignedIntentHash for ValidatedNotarizedTransactionV1 {
    fn signed_intent_hash(&self) -> SignedIntentHash {
        self.prepared.signed_intent_hash()
    }
}

impl HasNotarizedTransactionHash for ValidatedNotarizedTransactionV1 {
    fn notarized_transaction_hash(&self) -> NotarizedTransactionHash {
        self.prepared.notarized_transaction_hash()
    }
}

impl ValidatedNotarizedTransactionV1 {
    pub fn get_executable<'a>(&'a self) -> Executable<'a> {
        let intent = &self.prepared.signed_intent.intent;
        let header = &intent.header.inner;
        let intent_hash = intent.intent_hash();
        let summary = &self.prepared.summary;

        Executable::new(
            &self.encoded_instructions,
            &intent.instructions.references,
            &intent.blobs.blobs_by_hash,
            ExecutionContext {
                transaction_hash: intent_hash.into_hash(),
                payload_size: summary.effective_length,
                auth_zone_params: AuthZoneParams {
                    initial_proofs: AuthAddresses::signer_set(&self.signer_keys),
                    virtual_resources: BTreeSet::new(),
                },
                fee_payment: FeePayment {
                    tip_percentage: intent.header.inner.tip_percentage,
                    free_credit_in_xrd: 0,
                },
                runtime_validations: vec![
                    RuntimeValidation::IntentHashUniqueness {
                        intent_hash: intent_hash.into_hash(),
                    }
                    .enforced(),
                    RuntimeValidation::WithinEpochRange {
                        start_epoch_inclusive: header.start_epoch_inclusive,
                        end_epoch_exclusive: header.end_epoch_exclusive,
                    }
                    .enforced(),
                ],
                pre_allocated_ids: index_set_new(),
            },
        )
    }
}