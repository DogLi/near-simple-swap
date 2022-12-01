use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::U128;
use near_units::{parse_gas, parse_near};
use serde_json::{json, Value};
use workspaces::{Account, Contract};

const DEFI_WASM_FILEPATH: &str = "../res/defi.wasm";
const FT_WASM_FILEPATH: &str = "../res/fungible_token.wasm";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initiate environemnt
    let worker = workspaces::sandbox().await?;

    // deploy contracts
    let defi_wasm = std::fs::read(DEFI_WASM_FILEPATH)?;
    let defi_contract = worker.dev_deploy(&defi_wasm).await?;

    let ft_wasm_a = std::fs::read(FT_WASM_FILEPATH)?;
    let ft_contract_a = worker.dev_deploy(&ft_wasm_a).await?;

    let ft_wasm_b = std::fs::read(FT_WASM_FILEPATH)?;
    let ft_contract_b = worker.dev_deploy(&ft_wasm_b).await?;

    // create accounts
    let owner = worker.root_account().unwrap();

    let defi_detail = defi_contract.as_account().view_account().await?;
    println!("defi account details: {:?}", defi_detail);

    let alice = owner
        .create_subaccount("alice")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;
    let bob = owner
        .create_subaccount("bob")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;
    println!("alice address: {:?}", alice.id());
    println!("bob address: {:?}", bob.id());
    println!("defi contract address: {:?}", defi_contract.id());

    // Initialize contract of token A
    let result = ft_contract_a
        .call("new_default_meta")
        .args_json(json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("1,000,000,000 N").to_string(),
            "token_name": "fungible token A".to_string(),
            "symbol": "TokenA".to_string(),
            "decimals": 8,
        }))
        .transact()
        .await?;
    assert!(result.is_success());
    let result = ft_contract_a.call("ft_metadata").args_json(json!({})).transact().await?;
    assert!(result.is_success());
    let metadata_a: FungibleTokenMetadata = result.json()?;
    println!("meta data a: symbol: {}, decimals:{:?}", metadata_a.symbol, metadata_a.decimals);

    // println!("meta_data of token A: {:?}", token_meta_data_a);

    // Initialize contract of token B
    let result = ft_contract_b
        .call("new_default_meta")
        .args_json(serde_json::json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("1,000,000,000 N").to_string(),
            "token_name": "fungible token B".to_string(),
            "symbol": "TokenB".to_string(),
            "decimals": 8,
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = ft_contract_b.call("ft_metadata").args_json(json!({})).transact().await?;
    assert!(result.is_success());
    let metadata_b: FungibleTokenMetadata = result.json()?;
    println!("meta data b: symbol: {}, decimals:{:?}", metadata_b.symbol, metadata_b.decimals);

    // Initialize contract of defi
    let result = defi_contract
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": defi_contract.id(),
            "token_a": {
                "address": ft_contract_a.id(),
                "ticker": "0.1",
            },
            "token_b": {
                "address": ft_contract_b.id(),
                "ticker": "0.01",
            }
        }))
        .transact()
        .await?;
    // println!("{:?}", result);
    assert!(result.is_success());

    // set the token info
    for (prefix, id) in [("token_a", ft_contract_a.id()), ("token_b", ft_contract_b.id())].into_iter() {
        let result = defi_contract
            .call("set_token_info")
            .args_json(json!({ "token_address": id }))
            .gas(parse_gas!("300 Tgas") as u64)
            .transact()
            .await?;
        assert!(result.is_success(), "set token info error");

        // register
        let result = defi_contract
            .as_account()
            .call(id, "storage_deposit")
            .args_json(serde_json::json!({
                "account_id": format!("{}.{}", prefix, defi_contract.id())
            }))
            .deposit(parse_near!("0.008 N"))
            .transact()
            .await?;
        assert!(
            result.is_success(),
            "register defi smart contract address in token contract failed"
        )
    }

    let token_info: Value = defi_contract
        .call("get_token_info")
        .args_json(json!({
            "symbol": "TokenA"
        }))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?
        .json()?;
    println!("get token info: {:?}", token_info);

    // register the defi contract address in token contract


    // transfer some token to alice and bob
    let amount1 = U128::from(parse_near!("1,000 N"));
    println!("transfer TokenA");
    transfer_balance(&owner, defi_contract.as_account(), &ft_contract_a, amount1).await?;
    transfer_balance(&owner, &alice, &ft_contract_a, amount1).await?;

    println!("transfer TokenB");
    transfer_balance(&owner, defi_contract.as_account(), &ft_contract_b, amount1).await?;
    transfer_balance(&owner, &alice, &ft_contract_b, amount1).await?;

    let pool_balance_a: U128 = defi_contract
        .as_account()
        .call(defi_contract.id(), "get_swap_token")
        .args_json(json!({"symbol": "TokenA"}))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?
        .json()?;
    println!("get swap TokenA: {:?}", pool_balance_a);

    // deposit  TokenA to Swap
    for symbol in ["TokenA", "TokenB"] {
        let result = defi_contract
            .as_account()
            .call(defi_contract.id(), "deposit_token")
            .args_json(json!({"symbol": symbol, "amount": "100"}))
            .deposit(1)
            .gas(parse_gas!("300 Tgas") as u64)
            .transact()
            .await?;
        println!("defi deposit is success: {:?}", result.is_success());
    }

    // get the TokenA
    let result: U128 = defi_contract
        .as_account()
        .call(defi_contract.id(), "get_swap_token")
        .args_json(json!({"symbol": "TokenA"}))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?
        .json()?;
    println!("TokenA in Swap: {:?}", result);


    // get the ratio
    let result: U128 = defi_contract
        .as_account()
        .call(defi_contract.id(), "get_token_ratio")
        .args_json(json!({}))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?
        .json()?;
    println!("get ratio: {:?}", result);

    // swap
    let result = alice
        .call(defi_contract.id(), "swap_token")
        .args_json(json!({
            "symbol": "TokenA",
            "amount": "10",
        }))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?;
    println!("swap result is success: {:?}", result.is_success());
    assert!(result.is_success());

    // get the TokenA again
    let result: U128 = defi_contract
        .as_account()
        .call(defi_contract.id(), "get_swap_token")
        .args_json(json!({"symbol": "TokenA"}))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?
        .json()?;
    println!("TokenA in Swap: {:?}", result);
    Ok(())
}

async fn transfer_balance(
    owner: &Account,
    to_user: &Account,
    contract: &Contract,
    transfer_amount: U128,
) -> anyhow::Result<()> {
    // register user
    let result = to_user
        .call(contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": to_user.id()
        }))
        .deposit(parse_near!("0.008 N"))
        .transact()
        .await?;
    assert!(result.is_success());

    // transfer ft
    let result = owner
        .call(contract.id(), "ft_transfer")
        .args_json(serde_json::json!({
            "receiver_id": to_user.id(),
            "amount": transfer_amount
        }))
        .deposit(1)
        .transact()
        .await?;
    assert!(result.is_success());

    let user_balance: U128 = owner
        .call(contract.id(), "ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": to_user.id()
        }))
        .transact()
        .await?
        .json()?;

    assert!(user_balance >= transfer_amount);
    println!(" ✅ transfer balance success");
    Ok(())
}
