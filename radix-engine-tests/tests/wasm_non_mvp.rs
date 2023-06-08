use paste::paste;
use radix_engine::types::*;
use radix_engine::vm::wasm::WasmModule;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

// Verify WASM sign-extensions, which were enabled by default to the wasm32 target
// since rust 1.70.0
// see: https://github.com/rust-lang/rust/issues/109807
macro_rules! assert_sign_extensions {
    ($type:expr, $instruction:expr, $input:expr, $output:expr) => {
        paste! {
            #[test]
            fn [<test_wasm_non_mvp_sign_extensions_ $type _ $instruction>]() {
                // Arrange
                let value_kind = BasicValueKind::[<$type:upper>].as_u8().to_string();
                let slice_len = (
                    1 +                           // prefix byte
                    1 +                           // value kind byte
                    std::mem::size_of::<$type>()  // value bytes
                ).to_string();
                let input = $input as $type;

                // Act
                let code = wat2wasm(&include_str!("wasm/sign_extensions.wat")
                        .replace("${base}", stringify!($type))
                        .replace("${instruction}", $instruction)
                        .replace("${initial}", &input.to_string())
                        .replace("${value_kind}", &value_kind)
                        .replace("${slice_len}", &slice_len));

                let mut test_runner = TestRunner::builder().build();
                let package_address = test_runner.publish_package(
                    code,
                    single_function_package_definition("Test", "f"),
                    BTreeMap::new(),
                    OwnerRole::None,
                );
                let manifest = ManifestBuilder::new()
                    .lock_fee(test_runner.faucet_component(), 10.into())
                    .call_function(package_address, "Test", "f", manifest_args!())
                    .build();
                let receipt = test_runner.execute_manifest(manifest, vec![]);

                // Assert
                let outcome: $type = receipt.expect_commit(true).output(1);
                assert_eq!(outcome, $output as $type);
            }
        }
    };
}

assert_sign_extensions!(i32, "extend8_s", 0x44332211, 0x11);
assert_sign_extensions!(i32, "extend16_s", 0x44332211, 0x2211);
assert_sign_extensions!(i64, "extend8_s", 0x665544332211, 0x11);
assert_sign_extensions!(i64, "extend16_s", 0x665544332211, 0x2211);
assert_sign_extensions!(i64, "extend32_s", 0x665544332211, 0x44332211);

#[test]
fn test_wasm_non_mvp_expect_sign_ext_from_rust_code() {
    // Arrange
    let (code, _) = Compile::compile("./tests/blueprints/wasm_non_mvp");

    assert!(WasmModule::init(&code).unwrap().contains_sign_ext_ops())
}
