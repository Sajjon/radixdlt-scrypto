use native_sdk::modules::access_rules::AccessRules;
use native_sdk::modules::metadata::Metadata;
use native_sdk::modules::royalty::ComponentRoyalty;
use radix_engine::errors::{RuntimeError, SystemError};
use radix_engine::kernel::kernel_api::{KernelNodeApi, KernelSubstateApi};
use radix_engine::system::system_callback::SystemLockData;
use radix_engine::types::*;
use radix_engine::vm::{OverridePackageCode, VmInvoke};
use radix_engine_interface::api::node_modules::royalty::{
    ComponentRoyaltySetInput, COMPONENT_ROYALTY_SET_ROYALTY_IDENT,
};
use radix_engine_interface::api::{
    ClientApi, FieldValue, LockFlags, ObjectModuleId, OBJECT_HANDLE_SELF,
};
use radix_engine_interface::blueprints::package::PackageDefinition;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

#[test]
fn should_not_be_able_to_call_royalty_methods_on_resource_manager() {
    // Arrange
    const BLUEPRINT_NAME: &str = "MyBlueprint";
    const CUSTOM_PACKAGE_CODE_ID: u64 = 1024;

    // Arrange
    #[derive(Clone)]
    struct TestInvoke;
    impl VmInvoke for TestInvoke {
        fn invoke<Y>(
            &mut self,
            _export_name: &str,
            input: &IndexedScryptoValue,
            api: &mut Y,
        ) -> Result<IndexedScryptoValue, RuntimeError>
        where
            Y: ClientApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<SystemLockData>,
        {
            let node_id = input.references()[0];
            let _ = api.call_method_advanced(
                &node_id,
                ObjectModuleId::Royalty,
                false,
                COMPONENT_ROYALTY_SET_ROYALTY_IDENT,
                scrypto_encode(&ComponentRoyaltySetInput {
                    method: "some_method".to_string(),
                    amount: RoyaltyAmount::Free,
                })
                .unwrap(),
            )?;

            Ok(IndexedScryptoValue::from_typed(&()))
        }
    }
    let mut test_runner = TestRunnerBuilder::new().build_with_native_vm_extension(
        OverridePackageCode::new(CUSTOM_PACKAGE_CODE_ID, TestInvoke),
    );
    let package_address = test_runner.publish_native_package(
        CUSTOM_PACKAGE_CODE_ID,
        PackageDefinition::new_functions_only_test_definition(
            BLUEPRINT_NAME,
            vec![("test", "test", false)],
        ),
    );
    let resource_address = test_runner
        .create_everything_allowed_non_fungible_resource(OwnerRole::Fixed(rule!(allow_all)));

    // Act
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee(test_runner.faucet_component(), 500u32)
            .call_function(
                package_address,
                BLUEPRINT_NAME,
                "test",
                manifest_args!(resource_address),
            )
            .build(),
        vec![],
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemError(SystemError::ObjectModuleDoesNotExist(
                ObjectModuleId::Royalty
            ))
        )
    });
}

fn should_not_be_able_to_call_royalty_methods_on_package() {}

#[test]
fn should_not_be_able_to_call_metadata_methods_on_frame_owned_object() {
    const BLUEPRINT_NAME: &str = "MyBlueprint";
    const CUSTOM_PACKAGE_CODE_ID: u64 = 1024;

    // Arrange
    #[derive(Clone)]
    struct TestInvoke;
    impl VmInvoke for TestInvoke {
        fn invoke<Y>(
            &mut self,
            export_name: &str,
            _input: &IndexedScryptoValue,
            api: &mut Y,
        ) -> Result<IndexedScryptoValue, RuntimeError>
        where
            Y: ClientApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<SystemLockData>,
        {
            match export_name {
                "test" => {
                    let node_id = api.new_simple_object(BLUEPRINT_NAME, vec![])?;
                    let _ = api.call_method_advanced(
                        &node_id,
                        ObjectModuleId::Metadata,
                        false,
                        METADATA_SET_IDENT,
                        scrypto_encode(&MetadataSetInput {
                            key: "key".to_string(),
                            value: MetadataValue::String("value".to_string()),
                        })
                        .unwrap(),
                    )?;
                    api.drop_object(&node_id)?;
                    Ok(IndexedScryptoValue::from_typed(&()))
                }
                _ => Ok(IndexedScryptoValue::from_typed(&())),
            }
        }
    }
    let mut test_runner = TestRunnerBuilder::new().build_with_native_vm_extension(
        OverridePackageCode::new(CUSTOM_PACKAGE_CODE_ID, TestInvoke),
    );
    let package_address = test_runner.publish_native_package(
        CUSTOM_PACKAGE_CODE_ID,
        PackageDefinition::new_functions_only_test_definition(
            BLUEPRINT_NAME,
            vec![("test", "test", false)],
        ),
    );

    // Act
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee(test_runner.faucet_component(), 500u32)
            .call_function(package_address, BLUEPRINT_NAME, "test", manifest_args!())
            .build(),
        vec![],
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemError(SystemError::ObjectModuleDoesNotExist(
                ObjectModuleId::Metadata
            ))
        )
    });
}

fn should_not_be_able_to_call_metadata_methods_on_child_object(globalized_parent: bool) {
    const BLUEPRINT_NAME: &str = "MyBlueprint";
    const CUSTOM_PACKAGE_CODE_ID: u64 = 1024;

    // Arrange
    #[derive(Clone)]
    struct TestInvoke {
        globalized_parent: bool,
    }
    impl VmInvoke for TestInvoke {
        fn invoke<Y>(
            &mut self,
            export_name: &str,
            _input: &IndexedScryptoValue,
            api: &mut Y,
        ) -> Result<IndexedScryptoValue, RuntimeError>
        where
            Y: ClientApi<RuntimeError> + KernelNodeApi + KernelSubstateApi<SystemLockData>,
        {
            match export_name {
                "test" => {
                    let child = api.new_simple_object(
                        BLUEPRINT_NAME,
                        vec![FieldValue::new(&Option::<Own>::None)],
                    )?;
                    let parent = api.new_simple_object(
                        BLUEPRINT_NAME,
                        vec![FieldValue::new(&Option::<Own>::Some(Own(child)))],
                    )?;

                    let parent_node_id = if self.globalized_parent {
                        let metadata = Metadata::create(api)?;
                        let access_rules = AccessRules::create(OwnerRole::None, btreemap!(), api)?;
                        let royalty =
                            ComponentRoyalty::create(ComponentRoyaltyConfig::Disabled, api)?;

                        let address = api.globalize(
                            btreemap!(
                                ObjectModuleId::Main => parent,
                                ObjectModuleId::Metadata => metadata.0,
                                ObjectModuleId::AccessRules => access_rules.0.0,
                                ObjectModuleId::Royalty => royalty.0,
                            ),
                            None,
                        )?;
                        address.into_node_id()
                    } else {
                        parent
                    };

                    api.call_method(&parent_node_id, "call_metadata_on_child", scrypto_args!())?;

                    Ok(IndexedScryptoValue::from_typed(&()))
                }
                "call_metadata_on_child" => {
                    let handle =
                        api.actor_open_field(OBJECT_HANDLE_SELF, 0u8, LockFlags::read_only())?;
                    let child: Option<Own> = api.field_read_typed(handle)?;

                    let _ = api.call_method_advanced(
                        &child.unwrap().0,
                        ObjectModuleId::Metadata,
                        false,
                        METADATA_SET_IDENT,
                        scrypto_encode(&MetadataSetInput {
                            key: "key".to_string(),
                            value: MetadataValue::String("value".to_string()),
                        })
                        .unwrap(),
                    )?;

                    Ok(IndexedScryptoValue::from_typed(&()))
                }
                _ => Ok(IndexedScryptoValue::from_typed(&())),
            }
        }
    }
    let mut test_runner = TestRunnerBuilder::new().build_with_native_vm_extension(
        OverridePackageCode::new(CUSTOM_PACKAGE_CODE_ID, TestInvoke { globalized_parent }),
    );
    let package_address = test_runner.publish_native_package(
        CUSTOM_PACKAGE_CODE_ID,
        PackageDefinition::new_with_field_test_definition(
            BLUEPRINT_NAME,
            vec![
                ("test", "test", false),
                ("call_metadata_on_child", "call_metadata_on_child", true),
            ],
        ),
    );

    // Act
    let receipt = test_runner.execute_manifest(
        ManifestBuilder::new()
            .lock_fee(test_runner.faucet_component(), 500u32)
            .call_function(package_address, BLUEPRINT_NAME, "test", manifest_args!())
            .build(),
        vec![],
    );

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemError(SystemError::ObjectModuleDoesNotExist(
                ObjectModuleId::Metadata
            ))
        )
    });
}

#[test]
fn should_not_be_able_to_call_metadata_methods_on_frame_owned_child_object() {
    should_not_be_able_to_call_metadata_methods_on_child_object(false);
}

#[test]
fn should_not_be_able_to_call_metadata_methods_on_globalized_child_object() {
    should_not_be_able_to_call_metadata_methods_on_child_object(true);
}
