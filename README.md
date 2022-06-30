Staking Pool Owner Proxy Contract
======

This contract can be deployed to an account that owns a staking pool to redistribute the validator service fee among a set of recipients.

How to use this contract
=====
 
- Deploy a staking pool on the NEAR blockchain, specify an `owner account` that you have access to.
- Deploy this contract to `owner account` and specify the list of `reward receivers` 
- Call the `withdraw` method on the `owner account`, this action will unstake & withdraw service fee received by pool and distribute it among the `reward receivers`. 

This can be used to donate on a regular basis a portion of the pool's earnings to charity funds such as https://unchain.fund.

###INIT
```rust
near call $CONTRACT_ID new '{"staking_pool_account_id": "'$STAKING_POOL'", "owner_id":"'$OWNER_ID'", "reward_receivers": [["account_1.testnet", {"numerator": 7, "denominator":10}], ["account_2.testnet", {"numerator": 3, "denominator":10}]]}' --accountId $CONTRACT_ID
```

This will send 30% of pool rewards to `account_1.testnet` and 70% to `account_2.testnet`.

###RESET
```rust
near call $CONTRACT_ID reset_reward_receivers '{"reward_receivers": [["account_1.testnet", {"numerator": 30, "denominator":100}], ["account_1.testnet", {"numerator": 70, "denominator":100}]]}' --accountId $OWNER_ID
```

###Distribute Rewards
```rust
near call $CONTRACT_ID withdraw '{}' --accountId $CONTRACT_ID --gas 200000000000000
```

###Run tests
```
cd contract
cargo run --example staking -- --nocapture
```

Additional info
====

- How to sun a NEAR node: [NEAR Validator Bootcamp](https://near-nodes.io/validator/validator-bootcamp)
- How to interact with the NEAR blockchain [NEAR CLI](https://github.com/near/near-cli)
- [NEAR Examples](https://near.dev)

Risks
====
The contract was not audited! Use at your own risk


