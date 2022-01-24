Staking Pool Owner Proxy Contract
======

#How to test

```
export STAKING_POOL=staked.pool.f863973.m0
export CONTRACT_ID=dev-1643062986167-58132871434987
export TARGET_ACCOUNT_ID=zavodil.testnet
```

###INIT 
```
near call $CONTRACT_ID new '{"staking_pool_account_id": "'$STAKING_POOL'", "rewards_target_account_id":"'$TARGET_ACCOUNT_ID'"}' --accountId $CONTRACT_ID
```

###STAKE
```
near call $STAKING_POOL deposit_and_stake '{}' --accountId $CONTRACT_ID --deposit 5 --gas 50000000000000
```

###DISTRIBUTE
```
near call $CONTRACT_ID distribute '{}' --accountId $CONTRACT_ID --gas 200000000000000
```

###CHECK
```
near view $CONTRACT_ID get_is_distribution_allowed '{}'
near view $CONTRACT_ID get_current_epoch_id '{}'
near view $CONTRACT_ID get_unpaid_rewards '{}'

near view $STAKING_POOL get_account_unstaked_balance '{"account_id":"'$CONTRACT_ID'"}'
near view $STAKING_POOL get_account '{"account_id":"'$CONTRACT_ID'"}'
```

###GAS PROFILING
```
http post https://rpc.testnet.near.org method=tx params:='["54mmch7n5nWzLtDdXxr25QRTfYNNGb4HBV7oNTVEHWiU", "'$CONTRACT_ID'"]' jsonrpc=2.0 id=123
```

