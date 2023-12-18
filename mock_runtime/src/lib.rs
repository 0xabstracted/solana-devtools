pub mod message_processing;
pub mod spl_programs;

use crate::spl_programs::SPL_PROGRAMS;
use solana_bpf_loader_program::syscalls::create_program_runtime_environment;
use solana_program::clock::Clock;
use solana_program::epoch_schedule::EpochSchedule;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::slot_hashes::SlotHashes;
use solana_program::stake_history::StakeHistory;
use solana_program_runtime::invoke_context::InvokeContext;
use solana_program_runtime::loaded_programs::{LoadedProgram, LoadedPrograms};
use solana_program_runtime::log_collector::LogCollector;
use solana_program_runtime::sysvar_cache::SysvarCache;
use solana_rbpf::vm::BuiltinProgram;
use solana_runtime::builtins::BUILTINS;
use solana_sdk::account::AccountSharedData;
use solana_sdk::feature_set::FeatureSet;
use solana_sdk::native_loader::create_loadable_account_for_test;
use solana_sdk::slot_history::Slot;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Mock the Solana runtime environment, and provide a means
/// of processing `SanitizedMessage` values for testing purposes.
/// In addition, there are no signature verifications on accounts flagged as a signer
/// in the `SanitizedMessage`.
pub struct MockSolanaRuntime {
    cached_accounts: HashMap<Pubkey, AccountSharedData>,
    sysvar_cache: SysvarCache,
    feature_set: Arc<FeatureSet>,
    environment: Arc<BuiltinProgram<InvokeContext<'static>>>,
    loaded_programs: LoadedPrograms,
    logger: Rc<RefCell<LogCollector>>,
}

impl MockSolanaRuntime {
    pub fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let feature_set = Arc::new(FeatureSet::all_enabled());
        let environment =
            create_program_runtime_environment(&feature_set, &Default::default(), false, false)?;

        let mut loaded_programs = LoadedPrograms::default();
        for builtin in BUILTINS {
            loaded_programs.replenish(
                builtin.program_id,
                Arc::new(LoadedProgram::new_builtin(
                    0,
                    builtin.name.len(),
                    builtin.entrypoint,
                )),
            );
        }

        let mut sysvar_cache = populated_sysvar_cache();
        let mut clock = Clock::default();
        clock.slot = 1;
        sysvar_cache.set_clock(clock);

        Ok(Self {
            cached_accounts: Default::default(),
            sysvar_cache,
            feature_set,
            environment: Arc::new(environment),
            loaded_programs,
            logger: LogCollector::new_ref(),
        })
    }

    pub fn new_with_spl_and_builtins() -> Result<Self, Box<dyn std::error::Error>> {
        let mut this = Self::new()?;
        let mut accounts = HashMap::new();
        for builtin in BUILTINS {
            accounts.insert(
                builtin.program_id,
                create_loadable_account_for_test(builtin.name),
            );
        }
        for spl_program in SPL_PROGRAMS.iter() {
            let (pubkey, account) = spl_program.into();
            accounts.insert(pubkey, account);
        }
        this.update_accounts(&accounts);
        Ok(this)
    }

    /// This getter includes program accounts and program data accounts
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<&AccountSharedData> {
        self.cached_accounts.get(pubkey)
    }

    /// When loading transactions, we want to unwrap to default. This allows
    /// for operations like `system_instruction::create_account`.
    pub fn get_account_or_default(&self, pubkey: &Pubkey) -> AccountSharedData {
        self.get_account(pubkey).cloned().unwrap_or_default()
    }

    pub fn update_account(&mut self, pubkey: Pubkey, act: AccountSharedData) {
        self.cached_accounts.insert(pubkey, act);
    }

    /// Updates programs first.
    pub fn update_accounts(&mut self, accounts: &HashMap<Pubkey, AccountSharedData>) {
        accounts.iter().for_each(|(pubkey, act)| {
            self.update_account(*pubkey, act.clone());
        })
    }

    pub fn update_clock(&mut self, slot: Option<Slot>, unix_timestamp: Option<i64>) {
        let mut clock: Clock = (*self.sysvar_cache.get_clock().unwrap()).clone();
        if let Some(slot) = slot {
            clock.slot = slot;
        }
        if let Some(unix_timestamp) = unix_timestamp {
            clock.unix_timestamp = unix_timestamp;
        }
        self.sysvar_cache.set_clock(clock);
    }
}

fn populated_sysvar_cache() -> SysvarCache {
    let mut cache = SysvarCache::default();
    cache.set_clock(Clock::default());
    cache.set_rent(Rent::default());
    cache.set_epoch_schedule(EpochSchedule::default());
    cache.set_stake_history(StakeHistory::default());
    cache.set_slot_hashes(SlotHashes::default());
    cache
}
