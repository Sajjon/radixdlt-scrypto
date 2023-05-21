use radix_engine::errors::{ModuleError, RuntimeError, SystemError};
use radix_engine::system::system_modules::auth::AuthError;
use radix_engine::transaction::TransactionReceipt;
use radix_engine::types::*;
use radix_engine_interface::api::node_modules::auth::AuthAddresses;
use radix_engine_interface::blueprints::resource::FromPublicKey;
use radix_engine_interface::rule;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;
use transaction::ecdsa_secp256k1::EcdsaSecp256k1PrivateKey;
use transaction::model::TransactionManifest;

#[test]
fn initial_cyclic_authority_should_not_be_allowed() {
    let test_vectors = vec![
        {
            let mut authority_rules = Roles::new();
            authority_rules.define_role(
                "deposit_funds_auth",
                rule!(require("deposit_funds_auth")),
                rule!(deny_all),
            );
            authority_rules.define_role("borrow_faunds_auth", rule!(deny_all), rule!(deny_all));
            authority_rules
        },
        {
            let mut authority_rules = Roles::new();
            authority_rules.define_role(
                "deposit_funds_auth",
                rule!(require("borrow_funds_auth")),
                rule!(deny_all),
            );
            authority_rules.define_role(
                "borrow_faunds_auth",
                rule!(require("deposit_funds_auth")),
                rule!(deny_all),
            );

            authority_rules
        },
    ];

    // Arrange
    for authority_rules in test_vectors {
        let mut test_runner = TestRunner::builder().build();

        // Act
        let receipt =
            MutableAccessRulesTestRunner::create_component(authority_rules, &mut test_runner);

        // Assert
        receipt.expect_specific_failure(|e| {
            matches!(
                e,
                &RuntimeError::ModuleError(ModuleError::AuthError(AuthError::CycleCheckError(..)))
            )
        });
    }
}

#[test]
fn setting_circular_authority_rule_should_fail() {
    // Arrange
    let mut authority_rules = Roles::new();
    authority_rules.define_role("deposit_funds_auth", rule!(allow_all), rule!(allow_all));
    authority_rules.define_role("borrow_funds_auth", rule!(allow_all), rule!(allow_all));
    let mut test_runner = MutableAccessRulesTestRunner::new(authority_rules);

    // Act
    let receipt = test_runner.set_authority_rule(
        AuthorityKey::new("deposit_funds_auth"),
        rule!(require("deposit_funds")),
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            &RuntimeError::ModuleError(ModuleError::AuthError(AuthError::CycleCheckError(..)))
        )
    });
}

#[test]
fn access_rules_method_auth_can_not_be_mutated_when_locked() {
    // Arrange
    let mut authority_rules = Roles::new();
    authority_rules.define_role("borrow_funds_auth", rule!(allow_all), rule!(allow_all));
    authority_rules.define_role(
        "deposit_funds_auth",
        rule!(require(RADIX_TOKEN)),
        rule!(deny_all),
    );
    let mut test_runner = MutableAccessRulesTestRunner::new(authority_rules);

    // Act
    let receipt =
        test_runner.set_authority_rule(AuthorityKey::new("deposit_funds_auth"), rule!(allow_all));

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(e, RuntimeError::ModuleError(ModuleError::AuthError(..)))
    });
}

#[test]
fn access_rules_method_auth_cant_be_mutated_when_required_proofs_are_not_present() {
    // Arrange
    let private_key = EcdsaSecp256k1PrivateKey::from_u64(709).unwrap();
    let public_key = private_key.public_key();
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    let mut authority_rules = Roles::new();
    authority_rules.define_role("borrow_funds_auth", rule!(allow_all), rule!(allow_all));
    authority_rules.define_role(
        "deposit_funds_auth",
        rule!(require(RADIX_TOKEN)),
        rule!(require(virtual_badge_non_fungible_global_id.clone())),
    );
    let mut test_runner = MutableAccessRulesTestRunner::new(authority_rules.clone());

    // Act
    let receipt =
        test_runner.set_authority_rule(AuthorityKey::new("deposit_funds"), rule!(allow_all));

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(e, RuntimeError::ModuleError(ModuleError::AuthError(..)))
    });
}

#[test]
fn access_rules_method_auth_cant_be_locked_when_required_proofs_are_not_present() {
    // Arrange
    let private_key = EcdsaSecp256k1PrivateKey::from_u64(709).unwrap();
    let public_key = private_key.public_key();
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    let mut authority_rules = Roles::new();
    authority_rules.define_role("borrow_funds_auth", rule!(allow_all), rule!(allow_all));
    authority_rules.define_role(
        "deposit_funds_auth",
        rule!(require(RADIX_TOKEN)),
        rule!(require(virtual_badge_non_fungible_global_id.clone())),
    );
    let mut test_runner = MutableAccessRulesTestRunner::new(authority_rules);

    // Act
    let receipt = test_runner.lock_group_auth(AuthorityKey::new("deposit_funds_auth"));

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(e, RuntimeError::ModuleError(ModuleError::AuthError(..)))
    });
}

#[test]
fn access_rules_method_auth_can_be_mutated_when_required_proofs_are_present() {
    // Arrange
    let private_key = EcdsaSecp256k1PrivateKey::from_u64(709).unwrap();
    let public_key = private_key.public_key();
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    let mut authority_rules = Roles::new();
    authority_rules.define_role("borrow_funds_auth", rule!(allow_all), rule!(allow_all));
    authority_rules.define_role(
        "deposit_funds_auth",
        rule!(require(RADIX_TOKEN)),
        rule!(require(virtual_badge_non_fungible_global_id.clone())),
    );
    let mut test_runner = MutableAccessRulesTestRunner::new(authority_rules);

    // Act
    test_runner.add_initial_proof(virtual_badge_non_fungible_global_id);
    let receipt =
        test_runner.set_authority_rule(AuthorityKey::new("deposit_funds_auth"), rule!(allow_all));

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn access_rules_method_auth_can_be_locked_when_required_proofs_are_present() {
    // Arrange
    let private_key = EcdsaSecp256k1PrivateKey::from_u64(709).unwrap();
    let public_key = private_key.public_key();
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    let mut authority_rules = Roles::new();
    authority_rules.define_role("borrow_funds_auth", rule!(allow_all), rule!(allow_all));
    authority_rules.define_role(
        "deposit_funds_auth",
        rule!(require(RADIX_TOKEN)),
        rule!(require(virtual_badge_non_fungible_global_id.clone())),
    );
    let mut test_runner = MutableAccessRulesTestRunner::new(authority_rules);
    test_runner.add_initial_proof(virtual_badge_non_fungible_global_id);

    // Act
    let receipt = test_runner.lock_group_auth(AuthorityKey::new("deposit_funds_auth"));

    // Assert
    receipt.expect_commit_success();

    // Act
    let receipt =
        test_runner.set_authority_rule(AuthorityKey::new("deposit_funds_auth"), rule!(allow_all));

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(e, RuntimeError::ModuleError(ModuleError::AuthError(..)))
    });
}

fn component_access_rules_can_be_mutated_through_manifest(to_rule: AccessRule) {
    // Arrange
    let private_key = EcdsaSecp256k1PrivateKey::from_u64(709).unwrap();
    let public_key = private_key.public_key();
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    let mut authority_rules = Roles::new();
    authority_rules.define_role(
        "deposit_funds_auth",
        rule!(require("owner")),
        rule!(require("owner")),
    );
    authority_rules.define_role(
        "borrow_funds_auth",
        rule!(require("owner")),
        rule!(require("owner")),
    );
    authority_rules.set_owner_authority(
        rule!(require(RADIX_TOKEN)),
        rule!(require(virtual_badge_non_fungible_global_id.clone())),
    );

    let mut test_runner = MutableAccessRulesTestRunner::new(authority_rules);
    test_runner.add_initial_proof(virtual_badge_non_fungible_global_id.clone());

    // Act
    let receipt = test_runner.execute_manifest(
        MutableAccessRulesTestRunner::manifest_builder()
            .set_authority_access_rule(
                test_runner.component_address.into(),
                ObjectKey::SELF,
                AuthorityKey::new("owner"),
                to_rule,
            )
            .build(),
    );

    // Assert
    receipt.expect_commit_success();
    let receipt = test_runner.borrow_funds();
    receipt.expect_specific_failure(|e| {
        matches!(e, RuntimeError::ModuleError(ModuleError::AuthError(..)))
    });
}

#[test]
fn component_access_rules_can_be_mutated_to_deny_all_through_manifest() {
    component_access_rules_can_be_mutated_through_manifest(rule!(deny_all));
}

#[test]
fn component_access_rules_can_be_mutated_to_fungible_resource_through_manifest() {
    component_access_rules_can_be_mutated_through_manifest(rule!(require(RADIX_TOKEN)));
}

#[test]
fn component_access_rules_can_be_mutated_to_non_fungible_resource_through_manifest() {
    let non_fungible_global_id = AuthAddresses::system_role();
    component_access_rules_can_be_mutated_through_manifest(rule!(require(non_fungible_global_id)));
}

#[test]
fn assert_access_rule_through_component_when_not_fulfilled_fails() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/access_rules");
    let component_address = {
        let manifest = ManifestBuilder::new()
            .call_function(
                package_address,
                "AssertAccessRule".into(),
                "new",
                manifest_args!(),
            )
            .build();

        let receipt = test_runner.execute_manifest_ignoring_fee(manifest, []);
        receipt.expect_commit_success();

        receipt.expect_commit(true).new_component_addresses()[0]
    };

    // Act
    let manifest = ManifestBuilder::new()
        .call_method(
            component_address,
            "assert_access_rule",
            manifest_args!(rule!(require(RADIX_TOKEN))),
        )
        .build();

    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, []);

    // Assert
    receipt.expect_specific_failure(|error: &RuntimeError| {
        matches!(
            error,
            RuntimeError::SystemError(SystemError::AssertAccessRuleFailed)
        )
    })
}

#[test]
fn assert_access_rule_through_component_when_fulfilled_succeeds() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let (public_key, _, account) = test_runner.new_account(false);
    let package_address = test_runner.compile_and_publish("./tests/blueprints/access_rules");

    let component_address = {
        let manifest = ManifestBuilder::new()
            .call_function(
                package_address,
                "AssertAccessRule".into(),
                "new",
                manifest_args!(),
            )
            .build();

        let receipt = test_runner.execute_manifest_ignoring_fee(
            manifest,
            [NonFungibleGlobalId::from_public_key(&public_key)],
        );
        receipt.expect_commit_success();

        receipt.expect_commit(true).new_component_addresses()[0]
    };

    let manifest = ManifestBuilder::new()
        .create_proof_from_account(account, RADIX_TOKEN)
        .call_method(
            component_address,
            "assert_access_rule",
            manifest_args!(rule!(require(RADIX_TOKEN))),
        )
        .build();

    // Act
    let receipt = test_runner.execute_manifest_ignoring_fee(
        manifest,
        [NonFungibleGlobalId::from_public_key(&public_key)],
    );

    // Assert
    receipt.expect_commit_success();
}

struct MutableAccessRulesTestRunner {
    test_runner: TestRunner,
    component_address: ComponentAddress,
    initial_proofs: BTreeSet<NonFungibleGlobalId>,
}

impl MutableAccessRulesTestRunner {
    const BLUEPRINT_NAME: &'static str = "MutableAccessRulesComponent";

    pub fn create_component(
        authority_rules: Roles,
        test_runner: &mut TestRunner,
    ) -> TransactionReceipt {
        let package_address = test_runner.compile_and_publish("./tests/blueprints/access_rules");

        let manifest = ManifestBuilder::new()
            .call_function(
                package_address,
                Self::BLUEPRINT_NAME,
                "new",
                manifest_args!(authority_rules),
            )
            .build();
        test_runner.execute_manifest_ignoring_fee(manifest, vec![])
    }

    pub fn new(authority_rules: Roles) -> Self {
        let mut test_runner = TestRunner::builder().build();
        let receipt = Self::create_component(authority_rules, &mut test_runner);
        let component_address = receipt.expect_commit(true).new_component_addresses()[0];

        Self {
            test_runner,
            component_address,
            initial_proofs: BTreeSet::new(),
        }
    }

    pub fn add_initial_proof(&mut self, initial_proof: NonFungibleGlobalId) {
        self.initial_proofs.insert(initial_proof);
    }

    pub fn set_authority_rule(
        &mut self,
        authority_key: AuthorityKey,
        access_rule: AccessRule,
    ) -> TransactionReceipt {
        let manifest = Self::manifest_builder()
            .set_authority_access_rule(
                self.component_address.into(),
                ObjectKey::SELF,
                authority_key,
                access_rule,
            )
            .build();
        self.execute_manifest(manifest)
    }

    pub fn lock_group_auth(&mut self, authority_key: AuthorityKey) -> TransactionReceipt {
        let manifest = Self::manifest_builder()
            .set_authority_mutability(
                self.component_address.into(),
                ObjectKey::SELF,
                authority_key,
                AccessRule::DenyAll,
            )
            .build();
        self.execute_manifest(manifest)
    }

    pub fn borrow_funds(&mut self) -> TransactionReceipt {
        let manifest = Self::manifest_builder()
            .call_method(self.component_address, "borrow_funds", manifest_args!())
            .build();
        self.execute_manifest(manifest)
    }

    pub fn manifest_builder() -> ManifestBuilder {
        ManifestBuilder::new()
    }

    pub fn execute_manifest(&mut self, manifest: TransactionManifest) -> TransactionReceipt {
        self.test_runner
            .execute_manifest_ignoring_fee(manifest, self.initial_proofs.clone())
    }
}
