# Require: 
# 1. install juno on branch oracle
# 2. run node
# 3. query node address (junod keys list) and staking validators (junod q staking validators)
# 4. update addresses and keying in price-feeder/config.toml 

KEY="mykey"
DENOM='stake'
CHAIN_ID="test-1"
RPC='http://localhost:26657/'
REST='http://localhost:1317/'
TXFLAG="--gas-prices 0.025$DENOM --gas auto --gas-adjustment 1.3 -y -b block --chain-id $CHAIN_ID --node $RPC"
QFLAG="--chain-id $CHAIN_ID --node $RPC"
VALIDATOR=$(junod keys show $KEY -a)

junod tx wasm ../../store artifacts/oracle_querier.wasm --from $VALIDATOR $TXFLAG
CODE_ID=$(junod q wasm list-code --reverse --output json $QFLAG| jq -r '.code_infos[0].code_id')
echo $CODE_ID
junod tx wasm instantiate $CODE_ID  '{}' --from $VALIDATOR --label "oracle"  $TXFLAG  --no-admin 
CONTRACT_ADDR=$(junod q wasm list-contract-by-code 1 --output json $QFLAG | jq -r '.contracts[-1]')
echo $CONTRACT_ADDR
junod tx wasm execute $CONTRACT_ADDR '{"get_exchange_rate": { "denom" : "JUNO" } }' --from $VALIDATOR $TXFLAG 
junod query wasm contract-state smart $CONTRACT_ADDR '{"exchange_rate":{"denom":"JUNO"}}' --output json $QFLAG