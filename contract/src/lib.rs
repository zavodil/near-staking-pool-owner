use near_sdk::{AccountId, Balance, env, ext_contract, Gas, log, near_bindgen, PanicOnDefault, Promise, PromiseOrValue, PromiseResult};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;

const NUM_EPOCHS_TO_UNLOCK: u64 = 4;
const NO_DEPOSIT: Balance = 0;
const STAKING_POOL_READ_GAS: Gas = Gas(25_000_000_000_000);
const ON_DISTRIBUTE_GAS: Gas = Gas(120_000_000_000_000);
const WITHDRAW_GAS: Gas = Gas(25_000_000_000_000);
const ON_WITHDRAW_GAS: Gas = Gas(60_000_000_000_000);
const UNSTAKE_ALL_GAS: Gas = Gas(30_000_000_000_000);

type EpochId = u64;

/// Interface for a staking contract
#[ext_contract(ext_staking_pool)]
pub trait StakingPoolContract {
   /* Unstakes all staked balance */
   fn unstake_all(&self);
   /* Returns the unstaked balance of the given account */
   fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128;
   /* Withdraws the non staked balance for given account */
   fn withdraw(&self, amount: U128);
}

#[ext_contract(ext_self)]
pub trait ExtContract {
   /* Callback from checking unstaked balance */
   fn on_get_account_unstaked_balance(&mut self, #[callback] unstaked_amount: U128) -> U128;
   /* Callback from staking rewards withdraw */
   fn on_withdraw(&mut self, unstaked_amount: U128);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
   staking_pool_account_id: AccountId,
   rewards_target_account_id: AccountId,
   last_epoch_height: EpochId,
}

#[near_bindgen]
impl Contract {
   #[init]
   pub fn new(staking_pool_account_id: AccountId, rewards_target_account_id: AccountId) -> Self {
      Self {
         staking_pool_account_id,
         rewards_target_account_id,
         last_epoch_height: 0,
      }
   }

   pub fn distribute(&mut self) -> PromiseOrValue<U128> {
      assert!(env::epoch_height() > self.last_epoch_height + NUM_EPOCHS_TO_UNLOCK, "ERR_TOO_EARLY");

      PromiseOrValue::Promise(
         ext_staking_pool::get_account_unstaked_balance(
            env::current_account_id(),
            self.staking_pool_account_id.clone(),
            NO_DEPOSIT,
            STAKING_POOL_READ_GAS,
         ).then(
            ext_self::on_get_account_unstaked_balance(
               env::current_account_id(),
               NO_DEPOSIT,
               ON_DISTRIBUTE_GAS,
            )
         ))
   }

   #[private]
   pub fn on_get_account_unstaked_balance(&mut self, #[callback] unstaked_amount: U128) -> U128 {
      if unstaked_amount.0 > 0 {
         ext_staking_pool::withdraw(
            unstaked_amount,
            self.staking_pool_account_id.clone(),
            NO_DEPOSIT,
            WITHDRAW_GAS,
         ).then(
            ext_self::on_withdraw(
               unstaked_amount,
               env::current_account_id(),
               NO_DEPOSIT,
               ON_WITHDRAW_GAS
            )
         );
      }

      unstaked_amount
   }

   #[private]
   pub fn on_withdraw(&mut self, unstaked_amount: U128) {
      self.last_epoch_height = env::epoch_height();

      if is_promise_success() {
         log!("Unstaked rewards: {}", unstaked_amount.0);
         Promise::new(self.rewards_target_account_id.clone()).transfer(unstaked_amount.0);
      }

      ext_staking_pool::unstake_all(
         self.staking_pool_account_id.clone(),
         NO_DEPOSIT,
         UNSTAKE_ALL_GAS,
      );
   }

   pub fn get_is_distribution_allowed(&self) -> bool {
      self.last_epoch_height + NUM_EPOCHS_TO_UNLOCK < env::epoch_height()
   }

   pub fn get_staking_pool(&self) -> AccountId {
      self.staking_pool_account_id.clone()
   }

   pub fn get_rewards_target(&self) -> AccountId {
      self.rewards_target_account_id.clone()
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

fn is_promise_success() -> bool {
   assert_eq!(env::promise_results_count(), 1, "This is a callback method");
   match env::promise_result(0) {
      PromiseResult::Successful(_) => true,
      _ => false,
   }
}
