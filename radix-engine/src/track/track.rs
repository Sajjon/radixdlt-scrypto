use crate::types::*;
use radix_engine_interface::api::substate_api::LockFlags;
use radix_engine_interface::types::*;
use radix_engine_stores::interface::{
    AcquireLockError, StateUpdate, StateUpdates, SubstateDatabase, SubstateStore,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Sbor)]
pub enum SubstateLockState {
    Read(usize),
    Write,
}

impl SubstateLockState {
    pub fn no_lock() -> Self {
        Self::Read(0)
    }
}

#[derive(Debug)]
pub enum ExistingMetaState {
    Loaded,
    Updated(Option<IndexedScryptoValue>),
}

#[derive(Debug)]
pub enum SubstateMetaState {
    New,
    Existing { state: ExistingMetaState },
}

#[derive(Debug)]
pub struct LoadedSubstate {
    substate: IndexedScryptoValue,
    lock_state: SubstateLockState,
    meta_state: SubstateMetaState,
}

/// Transaction-wide states and side effects
pub struct Track<'s> {
    substate_db: &'s dyn SubstateDatabase,
    loaded_substates: IndexMap<NodeId, IndexMap<ModuleId, IndexMap<SubstateKey, LoadedSubstate>>>,
    locks: IndexMap<u32, (NodeId, ModuleId, SubstateKey, LockFlags)>,
    next_lock_id: u32,
}

impl<'s> Track<'s> {
    pub fn new(substate_db: &'s dyn SubstateDatabase) -> Self {
        Self {
            substate_db,
            loaded_substates: index_map_new(),
            locks: index_map_new(),
            next_lock_id: 0,
        }
    }

    fn new_lock_handle(
        &mut self,
        node_id: &NodeId,
        module_id: ModuleId,
        substate_key: &SubstateKey,
        flags: LockFlags,
    ) -> u32 {
        let new_lock = self.next_lock_id;
        self.locks
            .insert(new_lock, (*node_id, module_id, substate_key.clone(), flags));
        self.next_lock_id += 1;
        new_lock
    }

    fn loaded_substate<'a>(
        loaded_substates: &'a IndexMap<
            NodeId,
            IndexMap<ModuleId, IndexMap<SubstateKey, LoadedSubstate>>,
        >,
        node_id: &NodeId,
        module_id: ModuleId,
        substate_key: &SubstateKey,
    ) -> Option<&'a LoadedSubstate> {
        loaded_substates
            .get(node_id)
            .and_then(|m| m.get(&module_id))
            .and_then(|m| m.get(substate_key))
    }

    fn loaded_substate_mut<'a>(
        loaded_substates: &'a mut IndexMap<
            NodeId,
            IndexMap<ModuleId, IndexMap<SubstateKey, LoadedSubstate>>,
        >,
        node_id: &NodeId,
        module_id: ModuleId,
        substate_key: &SubstateKey,
    ) -> Option<&'a mut LoadedSubstate> {
        loaded_substates
            .get_mut(node_id)
            .and_then(|m| m.get_mut(&module_id))
            .and_then(|m| m.get_mut(substate_key))
    }

    fn load_substate(
        &mut self,
        node_id: &NodeId,
        module_id: ModuleId,
        substate_key: &SubstateKey,
    ) -> Option<IndexedScryptoValue> {
        self.substate_db
            .get_substate(node_id, module_id, substate_key)
            .expect("Database misconfigured")
            .map(|e| IndexedScryptoValue::from_vec(e).expect("Failed to decode substate"))
    }

    fn add_loaded_substate(
        &mut self,
        node_id: &NodeId,
        module_id: ModuleId,
        substate_key: &SubstateKey,
        substate_value: IndexedScryptoValue,
    ) {
        self.loaded_substates
            .entry(*node_id)
            .or_default()
            .entry(module_id)
            .or_default()
            .insert(
                substate_key.clone(),
                LoadedSubstate {
                    substate: substate_value,
                    lock_state: SubstateLockState::no_lock(),
                    meta_state: SubstateMetaState::Existing {
                        state: ExistingMetaState::Loaded,
                    },
                },
            );
    }

    /// Reverts all non force write changes.
    ///
    /// Note that dependencies will never be reverted.
    pub fn revert_non_force_write_changes(&mut self) {
        self.loaded_substates.retain(|_, m| {
            m.retain(|_, m| {
                m.retain(|_, loaded| match &loaded.meta_state {
                    SubstateMetaState::Existing {
                        state: ExistingMetaState::Updated(Some(value)),
                    } => {
                        loaded.substate = value.clone();
                        true
                    }
                    _ => false,
                });
                !m.is_empty()
            });
            !m.is_empty()
        });
    }

    /// Finalizes changes captured by this substate store.
    ///
    ///  Returns the state changes and dependencies.
    pub fn finalize(self) -> StateUpdates {
        // TODO:
        // - Remove version from state updates
        // - Split read,
        // - Track dependencies

        let mut substate_changes: IndexMap<(NodeId, ModuleId, SubstateKey), StateUpdate> =
            index_map_new();
        for (node_id, modules) in self.loaded_substates {
            for (module_id, module) in modules {
                for (substate_key, loaded) in module {
                    let update = match loaded.meta_state {
                        SubstateMetaState::New => StateUpdate::Create(loaded.substate.into()),
                        SubstateMetaState::Existing { .. } => {
                            StateUpdate::Update(loaded.substate.into())
                        }
                    };
                    substate_changes.insert((node_id, module_id, substate_key.clone()), update);
                }
            }
        }

        StateUpdates { substate_changes }
    }
}

impl<'s> SubstateStore for Track<'s> {
    fn acquire_lock(
        &mut self,
        node_id: &NodeId,
        module_id: ModuleId,
        substate_key: &SubstateKey,
        flags: LockFlags,
    ) -> Result<u32, AcquireLockError> {
        // Load the substate from state track
        if Self::loaded_substate(&self.loaded_substates, node_id, module_id, substate_key).is_none()
        {
            let maybe_substate = self.load_substate(node_id, module_id, substate_key);
            if let Some(output) = maybe_substate {
                self.add_loaded_substate(node_id, module_id, substate_key, output);
            } else {
                return Err(AcquireLockError::NotFound(
                    *node_id,
                    module_id,
                    substate_key.clone(),
                ));
            }
        }

        // Check substate state
        let loaded_substate =
            Self::loaded_substate_mut(&mut self.loaded_substates, node_id, module_id, substate_key)
                .unwrap();
        if flags.contains(LockFlags::UNMODIFIED_BASE) {
            match loaded_substate.meta_state {
                SubstateMetaState::New => {
                    return Err(AcquireLockError::LockUnmodifiedBaseOnNewSubstate(
                        *node_id,
                        module_id,
                        substate_key.clone(),
                    ))
                }
                SubstateMetaState::Existing {
                    state: ExistingMetaState::Updated(..),
                } => {
                    return Err(AcquireLockError::LockUnmodifiedBaseOnOnUpdatedSubstate(
                        *node_id,
                        module_id,
                        substate_key.clone(),
                    ))
                }
                SubstateMetaState::Existing {
                    state: ExistingMetaState::Loaded,
                } => {}
            }
        }

        // Check read/write permission
        match loaded_substate.lock_state {
            SubstateLockState::Read(n) => {
                if flags.contains(LockFlags::MUTABLE) {
                    if n != 0 {
                        return Err(AcquireLockError::SubstateLocked(
                            *node_id,
                            module_id,
                            substate_key.clone(),
                        ));
                    }
                    loaded_substate.lock_state = SubstateLockState::Write;
                } else {
                    loaded_substate.lock_state = SubstateLockState::Read(n + 1);
                }
            }
            SubstateLockState::Write => {
                return Err(AcquireLockError::SubstateLocked(
                    *node_id,
                    module_id,
                    substate_key.clone(),
                ));
            }
        }

        Ok(self.new_lock_handle(node_id, module_id, substate_key, flags))
    }

    fn release_lock(&mut self, handle: u32) {
        let (node_id, module_id, substate_key, flags) =
            self.locks.remove(&handle).expect("Invalid lock handle");

        let loaded_substate = Self::loaded_substate_mut(
            &mut self.loaded_substates,
            &node_id,
            module_id,
            &substate_key,
        )
        .expect("Substate missing for valid lock handle");

        match loaded_substate.lock_state {
            SubstateLockState::Read(n) => {
                loaded_substate.lock_state = SubstateLockState::Read(n - 1)
            }
            SubstateLockState::Write => {
                loaded_substate.lock_state = SubstateLockState::no_lock();

                if flags.contains(LockFlags::FORCE_WRITE) {
                    match &mut loaded_substate.meta_state {
                        SubstateMetaState::Existing { state, .. } => {
                            *state =
                                ExistingMetaState::Updated(Some(loaded_substate.substate.clone()));
                        }
                        SubstateMetaState::New => {
                            panic!("Unexpected");
                        }
                    }
                } else {
                    match &mut loaded_substate.meta_state {
                        SubstateMetaState::New => {}
                        SubstateMetaState::Existing { state } => match state {
                            ExistingMetaState::Loaded => *state = ExistingMetaState::Updated(None),
                            ExistingMetaState::Updated(..) => {}
                        },
                    }
                }
            }
        }
    }

    fn read_substate(&self, handle: u32) -> &IndexedScryptoValue {
        let (node_id, module_id, substate_key, _flags) =
            self.locks.get(&handle).expect("Invalid lock handle");

        &Self::loaded_substate(&self.loaded_substates, node_id, *module_id, substate_key)
            .expect("Substate missing for valid lock handle")
            .substate
    }

    fn update_substate(&mut self, handle: u32, substate_value: IndexedScryptoValue) {
        let (node_id, module_id, substate_key, flags) =
            self.locks.get(&handle).expect("Invalid lock handle");

        if !flags.contains(LockFlags::MUTABLE) {
            panic!("No write permission for {}", handle);
        }

        Self::loaded_substate_mut(
            &mut self.loaded_substates,
            node_id,
            *module_id,
            substate_key,
        )
        .expect("Substate missing for valid lock handle")
        .substate = substate_value;
    }

    fn create_substate(
        &mut self,
        node_id: NodeId,
        module_id: ModuleId,
        substate_key: SubstateKey,
        substate_value: IndexedScryptoValue,
    ) {
        self.loaded_substates
            .entry(node_id)
            .or_default()
            .entry(module_id)
            .or_default()
            .insert(
                substate_key,
                LoadedSubstate {
                    substate: substate_value,
                    lock_state: SubstateLockState::no_lock(),
                    meta_state: SubstateMetaState::New,
                },
            );
    }

    fn list_substates(
        &mut self,
        _node_id: &NodeId,
        _module_id: ModuleId,
    ) -> Box<dyn Iterator<Item = (SubstateKey, IndexedScryptoValue)>> {
        todo!()
    }
}