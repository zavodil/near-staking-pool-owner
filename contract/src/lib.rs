use near_sdk::{log, env, near_bindgen, Balance, AccountId, BorshStorageKey, PanicOnDefault, Promise, Gas, ext_contract};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{U128};

const NO_DEPOSIT: Balance = 0;
const UNSTAKE_ALL_GAS: Gas = Gas(30_000_000_000_000);
const WITHDRAW_GAS: Gas = Gas(30_000_000_000_000);
const STAKING_POOL_READ_GAS: Gas = Gas(20_000_000_000_000);
const DISTRIBUTE_GAS: Gas = Gas(25_000_000_000_000);

type EpochId = u64;

/// Interface for a staking contract
#[ext_contract(ext_staking_pool)]
pub trait StakingPoolContract {
   /// Unstakes all staked balance
   fn unstake_all(&self);
   /// Returns the unstaked balance of the given account
   fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128;
   /// Withdraws the entire unstaked balance
   fn withdraw_all(&self);
}

#[ext_contract(ext_self)]
pub trait ExtContract {
   /// Callback from checking unstaked balance
   fn on_get_account_unstaked_balance(&mut self, #[callback] unstaked_amount: U128) -> U128;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
	staking_pool_account_id: AccountId,
   rewards_target_account_id: AccountId,
   last_epoch_height: EpochId,
   unstaked_rewards: UnorderedMap<EpochId, Balance>
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
   UnstakedRewards
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(staking_pool_account_id: AccountId, rewards_target_account_id: AccountId) -> Self {
        Self {
           staking_pool_account_id,
           rewards_target_account_id,
           last_epoch_height: 0,
           unstaked_rewards: UnorderedMap::new(StorageKey::UnstakedRewards)
        }
    }

   pub fn distribute(&mut self) {
      let current_epoch = env::epoch_height();
      assert!(current_epoch > self.last_epoch_height, "ERR_EPOCH_ALREADY_PROCESSED");

      self.last_epoch_height = current_epoch;

      ext_staking_pool::unstake_all(
         self.staking_pool_account_id.clone(),
         NO_DEPOSIT,
         UNSTAKE_ALL_GAS
      )
      .then(
         ext_staking_pool::get_account_unstaked_balance(
            env::current_account_id(),
            self.staking_pool_account_id.clone(),
            NO_DEPOSIT,
            STAKING_POOL_READ_GAS
         ))
      .then(
         ext_self::on_get_account_unstaked_balance(
            env::current_account_id(),
            NO_DEPOSIT,
            DISTRIBUTE_GAS
      ));
   }

   #[private]
   pub fn on_get_account_unstaked_balance(&mut self, #[callback] unstaked_amount: U128) -> U128 {
      let current_epoch = env::epoch_height();
      let epoch_to_withdraw = current_epoch + 4;
      if unstaked_amount.0 > 0 {
         self.unstaked_rewards.insert(&epoch_to_withdraw,  &(self.unstaked_rewards.get(&epoch_to_withdraw).unwrap_or_default() + unstaked_amount.0));
      }

      let unpaid_epochs: Vec<(EpochId, Balance)> =
         self.unstaked_rewards
            .iter()
            .filter(|(k, _v)| *k <= current_epoch)
            .collect();

      if !unpaid_epochs.is_empty() {
         let unpaid_rewards: Balance = unpaid_epochs.iter().map(|(k, v)| {
            self.unstaked_rewards.remove(k);
            v
         }).sum();

         if unpaid_rewards > 0 {
            log!("Unpaid rewards found: {}", unpaid_rewards);

            ext_staking_pool::withdraw_all(
               self.staking_pool_account_id.clone(),
               NO_DEPOSIT,
               WITHDRAW_GAS
            )
               .then(
                  Promise::new(self.rewards_target_account_id.clone())
                     .transfer(unpaid_rewards)
               );

            return U128::from(unpaid_rewards);
         }
      }

      U128::from(0)
   }

   pub fn get_is_distribute_allowed(&self) -> bool {
      self.last_epoch_height < env::epoch_height()
   }

   pub fn get_staking_pool(&self) -> AccountId {
      self.staking_pool_account_id.clone()
   }

   pub fn get_rewards_target(&self) -> AccountId {
      self.rewards_target_account_id.clone()
   }

   pub fn get_unpaid_rewards(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<(EpochId, U128)> {
      unordered_map_pagination(&self.unstaked_rewards, from_index, limit)
   }

   pub fn get_current_epoch_id(&self) -> EpochId {
      env::epoch_height()
   }
   
   #[private]
   pub fn set_staking_pool(&mut self, account_id: AccountId) {
      self.staking_pool_account_id = account_id;
   }

   #[private]
   pub fn set_rewards_target(&mut self, account_id: AccountId) {
      self.rewards_target_account_id = account_id;
   }
}

pub fn unordered_map_pagination<K, VV, V>(
   m: &UnorderedMap<K, VV>,
   from_index: Option<u64>,
   limit: Option<u64>,
) -> Vec<(K, V)>
   where
      K: BorshSerialize + BorshDeserialize,
      VV: BorshSerialize + BorshDeserialize,
      V: From<VV>,
{
   let keys = m.keys_as_vector();
   let values = m.values_as_vector();
   let from_index = from_index.unwrap_or(0);
   let limit = limit.unwrap_or_else(|| keys.len());
   (from_index..std::cmp::min(keys.len(), limit))
      .map(|index| (keys.get(index).unwrap(), values.get(index).unwrap().into()))
      .collect()
}
