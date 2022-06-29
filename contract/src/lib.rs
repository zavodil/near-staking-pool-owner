use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, is_promise_success, log, near_bindgen, require, AccountId,
    Balance, Gas, PanicOnDefault, Promise, Timestamp, EpochHeight
};

const STAKING_POOL_PING_GAS: Gas = Gas(50_000_000_000_000);
const STAKING_POOL_READ_GAS: Gas = Gas(5_000_000_000_000);
const ON_DISTRIBUTE_GAS: Gas = Gas(120_000_000_000_000);
const WITHDRAW_GAS: Gas = Gas(25_000_000_000_000);
const ON_WITHDRAW_GAS: Gas = Gas(60_000_000_000_000);
const UNSTAKE_ALL_GAS: Gas = Gas(50_000_000_000_000);
const NUM_EPOCHS_TO_UNLOCK: EpochHeight = 4;

/// Represents an account structure readable by humans.
#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StakingPoolAccount {
    pub account_id: AccountId,
    /// The unstaked balance that can be withdrawn or staked.
    pub unstaked_balance: U128,
    /// The amount balance staked at the current "stake" share price.
    pub staked_balance: U128,
    /// Whether the unstaked balance is available for withdrawal now.
    pub can_withdraw: bool,
}

/// Interface for a staking contract
#[ext_contract(ext_staking_pool)]
pub trait StakingPoolContract {
    /* Pings staking pool */
    fn ping(&mut self);
    /* Unstakes all staked balance */
    fn unstake_all(&mut self);
    /* Returns the unstaked balance of the given account */
    fn get_account(&self, account_id: AccountId);
    /* Withdraws the non staked balance for given account */
    fn withdraw(&mut self, amount: U128);
}

#[ext_contract(ext_self)]
pub trait ExtContract {
    /* Callback from checking unstaked balance */
    fn on_get_account(&mut self, #[callback] account: StakingPoolAccount);
    /* Callback from staking rewards withdraw */
    fn on_withdraw(&mut self, unstaked_amount: U128, unstake_all: bool);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Contract {
    staking_pool_account_id: AccountId,
    owner_id: AccountId,
    reward_receivers: Vec<(AccountId, RewardFeeFraction)>,
    next_distribution_epoch: EpochHeight,
    #[serde(with = "u128_dec_format")]
    rewards_received: Balance,
    #[serde(with = "u64_dec_format")]
    last_reward_distribution: Timestamp,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct RewardFeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

impl RewardFeeFraction {
    pub fn assert_valid(&self) {
        assert_ne!(self.denominator, 0, "Denominator must be a positive number");
        assert!(
            self.numerator <= self.denominator,
            "The reward fee must be less or equal to 1"
        );
    }

    pub fn zero_fee() -> RewardFeeFraction {
        RewardFeeFraction {
            numerator: 0,
            denominator: 1
        }
    }

    pub fn multiply(&self, value: Balance) -> Balance {
        (U256::from(self.numerator) * U256::from(value) / U256::from(self.denominator)).as_u128()
    }

    pub fn add(&self, other: RewardFeeFraction) -> RewardFeeFraction {
        RewardFeeFraction {
            numerator: (self.numerator * other.denominator + other.numerator * self.denominator),
            denominator: (self.denominator * other.denominator)
        }
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        staking_pool_account_id: AccountId,
        owner_id: AccountId,
        reward_receivers: Vec<(AccountId, RewardFeeFraction)>,
    ) -> Self {

        assert_reward_receivers(&reward_receivers);

        Self {
            staking_pool_account_id,
            owner_id,
            reward_receivers,
            next_distribution_epoch: env::epoch_height() + NUM_EPOCHS_TO_UNLOCK,
            rewards_received: 0,
            last_reward_distribution: 0,
        }
    }

    pub fn reset_reward_receivers(&mut self, reward_receivers: Vec<(AccountId, RewardFeeFraction)>) {
        self.assert_owner();
        assert_reward_receivers(&reward_receivers);

        self.reward_receivers = reward_receivers;
    }

    pub fn get_info(&self) -> &Self {
        self
    }

    pub fn get_rewards_received(&self) -> Balance {
        self.rewards_received
    }


    // public method to distribute rewards
    pub fn withdraw(&mut self) -> Promise {
        assert!(self.next_distribution_epoch <= env::epoch_height(), "The unstaked balance is not yet available due to unstaking delay");

        ext_staking_pool::ext(self.staking_pool_account_id.clone())
            .with_static_gas(STAKING_POOL_PING_GAS)
            .ping()
        .then(ext_staking_pool::ext(self.staking_pool_account_id.clone())
            .with_static_gas(STAKING_POOL_READ_GAS)
            .get_account(env::current_account_id())
        )
        .then(ext_self::ext(env::current_account_id())
            .with_static_gas(ON_DISTRIBUTE_GAS)
            .on_get_account()
        )
    }

    #[private]
    pub fn on_get_account(&mut self, #[callback] account: StakingPoolAccount) {
        let unstake_all = account.staked_balance.0 > 0;
        if account.unstaked_balance.0 > 0 {
            if account.can_withdraw {
                log!(
                    "Withdrawing from staking pool: {}",
                    account.unstaked_balance.0
                );
                ext_staking_pool::ext(self.staking_pool_account_id.clone())
                    .with_static_gas(WITHDRAW_GAS)
                    .withdraw(account.unstaked_balance)
                .then(
                    ext_self::ext(env::current_account_id())
                        .with_static_gas(ON_WITHDRAW_GAS)
                        .on_withdraw(account.unstaked_balance, unstake_all)
                )
                .as_return();
            } else {
                log!("Awaiting unstaking. Nothing to do. Can't withdraw yet");
            }
        } else if unstake_all {
            self.internal_unstake_all();
        }
    }

    fn internal_unstake_all(&mut self) {
        log!("Unstaking all from staking pool");
        ext_staking_pool::ext(self.staking_pool_account_id.clone())
            .with_static_gas(UNSTAKE_ALL_GAS)
            .unstake_all()
        .as_return();
    }

    #[private]
    pub fn on_withdraw(&mut self, unstaked_amount: U128, unstake_all: bool) {
        require!(is_promise_success(), "Withdraw failed");
        log!(
            "Withdraw success! Received unstaked rewards: {}",
            unstaked_amount.0
        );
        self.rewards_received += unstaked_amount.0;

        // Send rewards
        for reward_receiver in &self.reward_receivers {
            transfer(reward_receiver.0.clone(), reward_receiver.1.multiply(unstaked_amount.0));
        }

        if unstake_all {
            self.internal_unstake_all();
        }
    }

    #[payable]
    pub fn donate(&mut self) {
        log!("Thank you for your {} yNEAR", env::attached_deposit())
    }
    pub fn get_current_env_data(&self) -> (u64, u64) {
        let now = env::block_timestamp();
        let eh = env::epoch_height();
        (now, eh)
    }


    pub fn get_staking_pool(&self) -> AccountId {
        self.staking_pool_account_id.clone()
    }
}

impl Contract {
    pub fn assert_owner(&self) {
        assert_eq!(
            &self.owner_id,
            &env::predecessor_account_id(),
            "Not an owner!"
        );
    }
}

uint::construct_uint!(
    pub struct U256(4);
);

pub(crate) mod u128_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

pub(crate) mod u64_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

fn assert_reward_receivers (reward_receivers: &Vec<(AccountId, RewardFeeFraction)>){
    let mut total_fee: RewardFeeFraction = RewardFeeFraction::zero_fee();
    for reward_receiver in reward_receivers {
        reward_receiver.1.assert_valid();
        total_fee = total_fee.add(reward_receiver.1.clone());
    }
    total_fee.assert_valid();
    assert_eq!(total_fee.numerator, total_fee.denominator, "ERR_ILLEGAL_REWARD_RECEIVERS");
}

fn transfer(account: AccountId, amount: Balance) {
    if amount > 0 {
        log!("Sending {} to {}", amount, account);
        Promise::new(account).transfer(amount);
    }
}