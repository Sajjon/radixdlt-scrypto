use crate::internal_prelude::*;

pub mod radiswap;
pub mod transfer_xrd;

pub fn get_builder_for_every_scenario() -> AllScenarios {
    AllScenarios { index: 0 }
}

pub struct AllScenarios {
    index: usize,
}

impl Iterator for AllScenarios {
    type Item = Box<dyn FnOnce(ScenarioCore) -> Box<dyn ScenarioInstance>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        match self.index {
            1 => Some(Box::new(|core| {
                Box::new(transfer_xrd::TransferXrdScenario::new(core))
            })),
            2 => Some(Box::new(|core| {
                Box::new(radiswap::RadiswapScenario::new(core))
            })),
            _ => None,
        }
    }
}
