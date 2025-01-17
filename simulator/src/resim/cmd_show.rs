use crate::resim::*;
use clap::Parser;
use radix_engine::types::*;
use radix_engine_stores::rocks_db::RocksdbSubstateStore;

/// Show an entity in the ledger state
#[derive(Parser, Debug)]
pub struct Show {
    /// The address of a package, component or resource manager
    pub address: String,
}

impl Show {
    pub fn run<O: std::io::Write>(&self, out: &mut O) -> Result<(), Error> {
        let scrypto_vm = ScryptoVm::<DefaultWasmEngine>::default();
        let native_vm = DefaultNativeVm::new();
        let vm = Vm::new(&scrypto_vm, native_vm);
        let mut substate_db = RocksdbSubstateStore::standard(get_data_dir()?);
        Bootstrapper::new(NetworkDefinition::simulator(), &mut substate_db, vm, false)
            .bootstrap_test_default();

        if let Ok(a) = SimulatorPackageAddress::from_str(&self.address) {
            dump_package(a.0, &substate_db, out).map_err(Error::LedgerDumpError)
        } else if let Ok(a) = SimulatorComponentAddress::from_str(&self.address) {
            dump_component(a.0, &substate_db, out).map_err(Error::LedgerDumpError)
        } else if let Ok(a) = SimulatorResourceAddress::from_str(&self.address) {
            dump_resource_manager(a.0, &substate_db, out).map_err(Error::LedgerDumpError)
        } else {
            Err(Error::InvalidId(self.address.clone()))
        }
    }
}
