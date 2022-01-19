Staking Pool Owner Proxy Contract
======

#How to test

```
export STAKING_POOL=staked.pool.f863973.m0
export CONTRACT_ID=dev-1642550364694-97849682442466
export TARGET_ACCOUNT_ID=zavodil.testnet
```

###INIT 
```
near call $CONTRACT_ID new '{"staking_pool_account_id": "'$STAKING_POOL'", "rewards_target_account_id":"'$TARGET_ACCOUNT_ID'"}' --accountId $CONTRACT_ID
```

###STAKE
```
near call $STAKING_POOL deposit_and_stake '{}' --accountId $CONTRACT_ID --deposit 2 --gas 50000000000000
```

###DISTRIBUTE
```
near call $CONTRACT_ID distribute '{}' --accountId $CONTRACT_ID --gas 200000000000000
```

###CHECK
```
near view $CONTRACT_ID get_current_epoch_id '{}'
near view $CONTRACT_ID get_unpaid_rewards '{}'
```
