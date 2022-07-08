use near_sdk::{Balance};
use serde_json::json;
use near_units::{parse_gas, parse_near};
use near_sdk::serde::{Deserialize, Serialize};
use workspaces::{Account, Worker};
use workspaces::network::Sandbox;

use workspaces::prelude::*;

const STAKING_KEY: &str = "KuTCtARNzxZQ3YvXDeLjx83FDqxv2SdQTSbiq876zR7";
const REWARD_1_ACCOUNT: &str = "reward_1";
const REWARD_1_FEE: u128 = 20;
const REWARD_2_ACCOUNT: &str = "reward_2";
const REWARD_2_FEE: u128 = 80;

const CONTRACT_WASM_FILEPATH: &str = "./../out/main.wasm";
const POOL_WASM_FILEPATH: &str = "./../out/staking_pool.wasm";

// wait ~ 4+ epoch
const DELTA_HEIGHT: u64 = 3000;

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct RewardFeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let worker: Worker<Sandbox> = workspaces::sandbox().await?;
    let owner = worker.root_account();
    let alice: Account = owner.create_subaccount(&worker, "alice")
        .initial_balance(parse_near!("11 N"))
        .transact()
        .await?
        .into_result()?;

    let reward_1: Account = owner.create_subaccount(&worker, REWARD_1_ACCOUNT)
        .transact()
        .await?
        .into_result()?;

    let reward_2: Account = owner.create_subaccount(&worker, REWARD_2_ACCOUNT)
        .transact()
        .await?
        .into_result()?;

    let init_balance_1_1 = reward_1.view_account(&worker).await?.balance;
    let init_balance_2_1 = reward_2.view_account(&worker).await?.balance;

    println!("alice account: {}, reward_1: {}, reward_2: {}", alice.id(), reward_1.id(), reward_2.id());

    let contract_wasm = std::fs::read(CONTRACT_WASM_FILEPATH)?;
    let contract = worker.dev_deploy(&contract_wasm).await?;

    let pool_wasm = std::fs::read(POOL_WASM_FILEPATH)?;
    let pool = worker.dev_deploy(&pool_wasm).await?;

    println!("Pool Account ID: {}", pool.id());
    println!("Owner Account ID: {}", contract.id());

    let _outcome_init_owner = contract
        .call(&worker, "new")
        .args_json(json!({
                "staking_pool_account_id": pool.id(),
                "owner_id": owner.id(),
                "reward_receivers": [
                    [reward_1.id(),
                        {"numerator": REWARD_1_FEE as u32, "denominator": 100u32}],
                    [reward_2.id(),
                        {"numerator": REWARD_2_FEE as u32, "denominator": 100u32}]
                ]
        }))?
        .gas(parse_gas!("300 T") as u64)
        .transact()
        .await?;
    //println!("init owner contract outcome: {:#?}", _outcome_init_owner);

    let _outcome_donate_rewards = owner
        .call(&worker, contract.id(), "donate")
        .deposit(parse_near!("100 N"))
        .transact()
        .await?;
    // println!("donate outcome: {:#?}", _outcome_donate_rewards);

    let _outcome_init_pool = pool
        .call(&worker, "new")
        .args_json(json!({
                "owner_id": contract.id(),
                "stake_public_key": STAKING_KEY,
                "reward_fee_fraction": {
                    "numerator": 10u32,
                    "denominator": 100u32
                }
        }))?
        .gas(parse_gas!("300 T") as u64)
        .transact()
        .await?;
    //println!("init pool outcome: {:#?}", _outcome_init_pool);

    let _outcome_deposit_and_stake = alice
        .call(&worker, pool.id(), "deposit_and_stake")
        .gas(parse_gas!("50 T") as u64)
        .deposit(parse_near!("10 N"))
        .transact()
        .await?;
    // println!("deposit_and_stake outcome: {:#?}", _outcome_deposit_and_stake);

    let _outcome_account: serde_json::Value = pool.call(&worker, "get_account")
        .args_json(json!({
                "account_id": alice.id(),
        }))?
        .view()
        .await?
        .json()?;
    //println!("account outcome: {:#?}", _outcome_account);

    let (_timestamp, epoch_height_1): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;
    //println!("timestamp = {}, epoch_height = {}", _timestamp, epoch_height_1);

    //let block_info = worker.view_latest_block().await?;
    //println!("BlockInfo pre-fast_forward {:?}", block_info);

    worker.fast_forward(DELTA_HEIGHT).await?;

    let (_timestamp, epoch_height_2): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;
    //println!("timestamp = {}, epoch_height = {}", _timestamp, epoch_height_2);

    println!("===> {} epochs passed", (epoch_height_2 - epoch_height_1).to_string());

    //let block_info = worker.view_latest_block().await?;
    //println!("BlockInfo post-fast_forward {:?}", block_info);

    let _outcome_account: serde_json::Value = pool.call(&worker, "get_account")
        .args_json(json!({
                "account_id": alice.id(),
        }))?
        .view()
        .await?
        .json()?;
    //println!("account outcome: {:#?}", outcome_account);

    let _outcome_withdraw = owner
        .call(&worker, contract.id(), "withdraw")
        .gas(parse_gas!("200 T") as u64)
        .transact()
        .await?;
    //println!("withdraw outcome: {:#?}", _outcome_withdraw);

    /*
    let outcome_withdraw_failed: CallExecutionDetails = owner
        .call(&worker, contract.id(), "withdraw")
        .gas(parse_gas!("200 T") as u64)
        .transact()
        .await?;
    println!("withdraw outcome: {:#?}", outcome_withdraw_failed);*/
    //assert!(outcome_withdraw_failed.is_failure());

    let (_timestamp, epoch_height_3): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;
    //println!("timestamp = {}, epoch_height = {}", _timestamp, epoch_height_3);

    //let block_info = worker.view_latest_block().await?;
    //println!("BlockInfo pre-fast_forward {:?}", block_info);

    // wait ~ 5 epoch
    worker.fast_forward(DELTA_HEIGHT).await?;

    let (_timestamp, epoch_height_4): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;
    //println!("timestamp = {}, epoch_height = {}", _timestamp, epoch_height_4);

    println!("===> {} epochs passed", (epoch_height_4 - epoch_height_3).to_string());

    //let block_info = worker.view_latest_block().await?;
    //println!("BlockInfo post-fast_forward {:?}", block_info);

    let _outcome_account: serde_json::Value = pool.call(&worker, "get_account")
        .args_json(json!({
                "account_id": alice.id(),
        }))?
        .view()
        .await?
        .json()?;
    // println!("account outcome: {:#?}", _outcome_account);

    let _outcome_withdraw = owner
        .call(&worker, contract.id(), "withdraw")
        .gas(parse_gas!("200 T") as u64)
        .transact()
        .await?;
    //println!("withdraw outcome: {:#?}", _outcome_withdraw);

    let rewards_received_1: Balance = contract.call(&worker, "get_rewards_received")
        .view()
        .await?
        .json::<Balance>()?;
    println!("rewards_received 1: {}", rewards_received_1);

    let init_balance_1_2 = reward_1.view_account(&worker).await?.balance;
    let init_balance_2_2 = reward_2.view_account(&worker).await?.balance;

    assert!(rewards_received_1 > 0);
    almost_eq(init_balance_1_2 - init_balance_1_1, rewards_received_1 / 100 * REWARD_1_FEE, 18, "rewards_received_1_1");
    almost_eq(init_balance_2_2 - init_balance_2_1, rewards_received_1 / 100 * REWARD_2_FEE, 18, "rewards_received_2_1");



    let (_timestamp, epoch_height_5): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;

    // wait ~ another 5 epoch
    worker.fast_forward(DELTA_HEIGHT).await?;


    let (_timestamp, epoch_height_6): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;
    //println!("timestamp = {}, epoch_height = {}", _timestamp, epoch_height_4);

    println!("===> {} epochs passed", (epoch_height_6 - epoch_height_5).to_string());

    //let block_info = worker.view_latest_block().await?;
    //println!("BlockInfo post-fast_forward {:?}", block_info);

    let _outcome_account: serde_json::Value = pool.call(&worker, "get_account")
        .args_json(json!({
                "account_id": alice.id(),
        }))?
        .view()
        .await?
        .json()?;
    // println!("account outcome: {:#?}", _outcome_account);

    let _outcome_withdraw = owner
        .call(&worker, contract.id(), "withdraw")
        .gas(parse_gas!("200 T") as u64)
        .transact()
        .await?;
    //println!("withdraw outcome: {:#?}", _outcome_withdraw);




    let (_timestamp, epoch_height_5): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;

    // wait ~ another 5 epoch
    worker.fast_forward(DELTA_HEIGHT).await?;

    let (_timestamp, epoch_height_6): (u64, u64) = contract
        .call(&worker, "get_current_env_data")
        .view()
        .await?
        .json()?;
    //println!("timestamp = {}, epoch_height = {}", _timestamp, epoch_height_4);

    println!("===> {} epochs passed", (epoch_height_6 - epoch_height_5).to_string());

    let _outcome_withdraw = owner
        .call(&worker, contract.id(), "withdraw")
        .gas(parse_gas!("200 T") as u64)
        .transact()
        .await?;
    //println!("withdraw outcome: {:#?}", _outcome_withdraw);


    let rewards_received_2: Balance = contract.call(&worker, "get_rewards_received")
        .view()
        .await?
        .json::<Balance>()?;
    println!("rewards_received 2: {}", rewards_received_2);

    let init_balance_1_3 = reward_1.view_account(&worker).await?.balance;
    let init_balance_2_3 = reward_2.view_account(&worker).await?.balance;

    assert!(rewards_received_2 > 0);
    almost_eq(init_balance_1_3 - init_balance_1_2, (rewards_received_2 - rewards_received_1) / 100 * REWARD_1_FEE, 15, "rewards_received_2_1");
    almost_eq(init_balance_2_3 - init_balance_2_2, (rewards_received_2 - rewards_received_1) / 100 * REWARD_2_FEE, 15, "rewards_received_2_2");


    Ok(())
}

pub fn almost_eq(a: u128, b: u128, prec: u32, name: &str) {
    let p = 10u128.pow(23 - prec);
    let ap = (a + p / 2) / p;
    let bp = (b + p / 2) / p;
    println!("almost_eq {}: {} <=> {}", name, ap, bp);
    assert_eq!(
        ap,
        bp,
        "{}",
        format!("Expected {} to eq {}, with precision {}", a, b, prec)
    );
}