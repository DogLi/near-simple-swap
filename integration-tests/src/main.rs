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
    let address_a = ft_contract_a.id().clone();

    let ft_wasm_b = std::fs::read(FT_WASM_FILEPATH)?;
    let ft_contract_b = worker.dev_deploy(&ft_wasm_b).await?;

    // create accounts
    let owner = worker.root_account().unwrap();

    let defi_detail = defi_contract.as_account().view_account().await?;
    println!("defi account details: {:?}", defi_detail);

    let defi_owner = owner
        .create_subaccount("defi_owner")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;
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
    println!("defi owner address: {:?}", defi_owner.id());
    println!("defi contract address: {:?}", defi_contract.id());
    // let charlie = owner
    //     .create_subaccount( "charlie")
    //     .initial_balance(parse_near!("30 N"))
    //     .transact()
    //     .await?
    //     .into_result()?;
    // let dave = owner
    //     .create_subaccount( "dave")
    //     .initial_balance(parse_near!("30 N"))
    //     .transact()
    //     .await?
    //     .into_result()?;

    // transfer some token to defi address

    // Initialize contract of token A
    let result = ft_contract_a
        .call("new_default_meta")
        .args_json(json!({
            "owner_id": owner.id(),
            "total_supply": parse_near!("1,000,000,000 N").to_string(),
            "token_name": "fungible token A".to_string(),
            "symbol": "TokenA".to_string(),
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
            "owner_id": defi_owner.id(),
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
    for id in [ft_contract_a.id(), ft_contract_b.id()].into_iter() {
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
                "account_id": defi_contract.id()
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

    // registe the defi contract address in token contract

    // transfer some token to alice and bob
    let amount1 = U128::from(parse_near!("1,000 N"));
    println!("transfer TokenA");
    transfer_balance(&owner, &defi_owner, &ft_contract_a, amount1).await?;
    transfer_balance(&owner, &defi_contract.as_account(), &ft_contract_a, amount1).await?;
    transfer_balance(&owner, &alice, &ft_contract_a, amount1).await?;

    println!("transfer TokenB");
    transfer_balance(&owner, &defi_owner, &ft_contract_b, amount1).await?;
    transfer_balance(&owner, &defi_contract.as_account(), &ft_contract_b, amount1).await?;
    transfer_balance(&owner, &alice, &ft_contract_b, amount1).await?;

    let pool_balance_a: U128 = defi_contract
        .as_account()
        .call(defi_contract.id(), "get_pool_token")
        .args_json(json!({"symbol": "TokenA"}))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?
        .json()?;
    println!("get TokenA: {:?}", pool_balance_a);

    let result = alice
        .call(defi_contract.id(), "swap_token")
        .args_json(json!({
            "symbol": "TokenA",
            "amount": "100",
        }))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?;
    println!("swap result: {:?}", result);
    assert!(result.is_success());

    let pool_balance_a: U128 = defi_contract
        .as_account()
        .call(defi_contract.id(), "get_pool_token")
        .args_json(json!({"symbol": "TokenA"}))
        .gas(parse_gas!("300 Tgas") as u64)
        .transact()
        .await?
        .json()?;
    println!("get TokenA: {:?}", pool_balance_a);

    // begin tests
    // test_total_supply(&owner, &ft_contract).await?;
    // test_simple_transfer(&owner, &alice, &ft_contract).await?;
    // test_can_close_empty_balance_account(&bob, &ft_contract).await?;
    // test_close_account_non_empty_balance(&alice, &ft_contract).await?;
    // test_close_account_force_non_empty_balance(&alice, &ft_contract).await?;
    // test_transfer_call_with_burned_amount(&owner, &charlie, &ft_contract, &defi_contract)
    //     .await?;
    // test_simulate_transfer_call_with_immediate_return_and_no_refund(
    //     &owner,
    //     &ft_contract,
    //     &defi_contract,
    // )
    // .await?;
    // test_transfer_call_when_called_contract_not_registered_with_ft(
    //     &owner,
    //     &dave,
    //     &ft_contract,
    // )
    // .await?;
    // test_transfer_call_promise_panics_for_a_full_refund(&owner, &alice, &ft_contract)
    //     .await?;
    Ok(())
}

// async fn test_total_supply(
//     owner: &Account,
//     contract: &Contract,
// ) -> anyhow::Result<()> {
//     let initial_balance = U128::from(parse_near!("1,000,000,000 N"));
//     let res: U128 = owner
//         .call(contract.id(), "ft_total_supply")
//         .args_json(json!({}))
//         .transact()
//         .await?
//         .json()?;
//     assert_eq!(res, initial_balance);
//     println!("      Passed ✅ test_total_supply");
//     Ok(())
// }
//
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

    let root_balance: U128 = owner
        .call(contract.id(), "ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": owner.id()
        }))
        .transact()
        .await?
        .json()?;

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

// async fn test_can_close_empty_balance_account(
//     user: &Account,
//     contract: &Contract,
// ) -> anyhow::Result<()> {
//     // register user
//     let result = user.call( contract.id(), "storage_deposit")
//         .args_json(serde_json::json!({
//             "account_id": user.id()
//         }))
//         .deposit(parse_near!("0.008 N"))
//         .transact()
//         .await?;
//     assert!(result.is_success());
//
//     let result: bool = user
//         .call( contract.id(), "storage_unregister")
//         .args_json(serde_json::json!({}))
//         .deposit(1)
//         .transact()
//         .await?
//         .json()?;
//
//     assert_eq!(result, true);
//     println!("      Passed ✅ test_can_close_empty_balance_account");
//     Ok(())
// }
//
// async fn test_close_account_non_empty_balance(
//     user_with_funds: &Account,
//     contract: &Contract,
// ) -> anyhow::Result<()> {
//     let result = user_with_funds
//         .call( contract.id(), "storage_unregister")
//         .args_json(serde_json::json!({}))
//         .deposit(1)
//         .transact()
//         .await?;
//     assert!(!result.is_success());
//     println!("      Passed ✅ test_close_account_non_empty_balance");
//     Ok(())
// }
//
// async fn test_close_account_force_non_empty_balance(
//     user_with_funds: &Account,
//     contract: &Contract,
// ) -> anyhow::Result<()> {
//     let result = user_with_funds
//         .call( contract.id(), "storage_unregister")
//         .args_json(serde_json::json!({"force": true }))
//         .deposit(1)
//         .transact()
//         .await?;
//
//     assert_eq!(true, result.is_success());
//     assert_eq!(
//         result.logs()[0],
//         format!(
//             "Closed @{} with {}",
//             user_with_funds.id(),
//             parse_near!("1,000 N") // alice balance from above transfer_amount
//         )
//     );
//     println!("      Passed ✅ test_close_account_force_non_empty_balance");
//     Ok(())
// }
//
// async fn test_transfer_call_with_burned_amount(
//     owner: &Account,
//     user: &Account,
//     ft_contract: &Contract,
//     defi_contract: &Contract,
// ) -> anyhow::Result<()> {
//     let transfer_amount_str = parse_near!("1,000,000 N").to_string();
//     let ftc_amount_str = parse_near!("1,000 N").to_string();
//
//     // register user
//     let result = owner
//         .call( ft_contract.id(), "storage_deposit")
//         .args_json(serde_json::json!({
//             "account_id": user.id()
//         }))
//         .deposit(parse_near!("0.008 N"))
//         .transact()
//         .await?;
//     assert!(result.is_success());
//
//     // transfer ft
//     let result = owner
//         .call( ft_contract.id(), "ft_transfer")
//         .args_json(serde_json::json!({
//             "receiver_id": user.id(),
//             "amount": transfer_amount_str
//         }))
//         .deposit(1)
//         .transact()
//         .await?;
//     assert!(result.is_success());
//
//     let result = user.call( ft_contract.id(), "ft_transfer_call")
//         .args_json(serde_json::json!({
//             "receiver_id": defi_contract.id(),
//             "amount": ftc_amount_str,
//             "msg": "0",
//         }))
//         .deposit(1)
//         .gas(parse_gas!("200 Tgas") as u64)
//         .transact()
//         .await?;
//     assert!(result.is_success());
//
//     let storage_result = user
//         .call( ft_contract.id(), "storage_unregister")
//         .args_json(serde_json::json!({"force": true }))
//         .deposit(1)
//         .transact()
//         .await?;
//
//     // assert new state
//     assert_eq!(
//         storage_result.logs()[0],
//         format!(
//             "Closed @{} with {}",
//             user.id(),
//             parse_near!("999,000 N") // balance after defi ft transfer
//         )
//     );
//
//     let total_supply: U128 = owner
//         .call( ft_contract.id(), "ft_total_supply")
//         .args_json(json!({}))
//         .transact()
//         .await?
//         .json()?;
//     assert_eq!(total_supply, U128::from(parse_near!("999,000,000 N")));
//
//     let defi_balance: U128 = owner
//         .call( ft_contract.id(), "ft_total_supply")
//         .args_json(json!({"account_id": defi_contract.id()}))
//         .transact()
//         .await?
//         .json()?;
//     assert_eq!(defi_balance, U128::from(parse_near!("999,000,000 N")));
//
//     println!("      Passed ✅ test_transfer_call_with_burned_amount");
//     Ok(())
// }
//
// async fn test_simulate_transfer_call_with_immediate_return_and_no_refund(
//     owner: &Account,
//     ft_contract: &Contract,
//     defi_contract: &Contract,
// ) -> anyhow::Result<()> {
//     let amount: u128 = parse_near!("100,000,000 N");
//     let amount_str = amount.to_string();
//     let owner_before_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id": owner.id()}))
//         .transact()
//         .await?
//         .json()?;
//     let defi_before_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id": defi_contract.id()}))
//         .transact()
//         .await?
//         .json()?;
//
//     let result = owner
//         .call( ft_contract.id(), "ft_transfer_call")
//         .args_json(serde_json::json!({
//             "receiver_id": defi_contract.id(),
//             "amount": amount_str,
//             "msg": "take-my-money"
//         }))
//         .deposit(1)
//         .gas(parse_gas!("200 Tgas") as u64)
//         .transact()
//         .await?;
//     assert!(result.is_success());
//
//     let owner_after_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id": owner.id()}))
//         .transact()
//         .await?
//         .json()?;
//     let defi_after_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id": defi_contract.id()}))
//         .transact()
//         .await?
//         .json()?;
//
//     assert_eq!(owner_before_balance.0 - amount, owner_after_balance.0);
//     assert_eq!(defi_before_balance.0 + amount, defi_after_balance.0);
//     println!("      Passed ✅ test_simulate_transfer_call_with_immediate_return_and_no_refund");
//     Ok(())
// }
//
// async fn test_transfer_call_when_called_contract_not_registered_with_ft(
//     owner: &Account,
//     user: &Account,
//     ft_contract: &Contract,
// ) -> anyhow::Result<()> {
//     let amount = parse_near!("10 N");
//     let amount_str = amount.to_string();
//     let owner_before_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id":  owner.id()}))
//         .transact()
//         .await?
//         .json()?;
//     let user_before_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id": user.id()}))
//         .transact()
//         .await?
//         .json()?;
//
//     let result = owner
//         .call( ft_contract.id(), "ft_transfer_call")
//         .args_json(serde_json::json!({
//             "receiver_id": user.id(),
//             "amount": amount_str,
//             "msg": "take-my-money",
//         }))
//         .deposit(1)
//         .gas(parse_gas!("200 Tgas") as u64)
//         .transact()
//         .await?;
//     let data: Value = result.json()?;
//     println!("test_transfer_call_when_called_contract_not_registered_with_ft: {:?}", data);
//     // {
//     //     Ok(_res) => {
//     //         panic!("Was able to transfer FT to an unregistered account");
//     //     }
//     //     Err(_err) => {
//     //         let owner_after_balance: U128 = ft_contract
//     //             .call( "ft_balance_of")
//     //             .args_json(json!({"account_id":  owner.id()}))
//     //             .transact()
//     //             .await?
//     //             .json()?;
//     //         let user_after_balance: U128 = ft_contract
//     //             .call( "ft_balance_of")
//     //             .args_json(json!({"account_id": user.id()}))
//     //             .transact()
//     //             .await?
//     //             .json()?;
//     //         assert_eq!(user_before_balance, user_after_balance);
//     //         assert_eq!(owner_before_balance, owner_after_balance);
//     //         println!(
//     //             "      Passed ✅ test_transfer_call_when_called_contract_not_registered_with_ft"
//     //         );
//     //     }
//     // }
//     Ok(())
// }
//
// async fn test_transfer_call_promise_panics_for_a_full_refund(
//     owner: &Account,
//     user: &Account,
//     ft_contract: &Contract,
// ) -> anyhow::Result<()> {
//     let amount = parse_near!("10 N");
//
//     // register user
//     let result = owner
//         .call( ft_contract.id(), "storage_deposit")
//         .args_json(serde_json::json!({
//             "account_id": user.id()
//         }))
//         .deposit(parse_near!("0.008 N"))
//         .transact()
//         .await?;
//     assert!(result.is_success());
//
//     let owner_before_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id":  owner.id()}))
//         .transact()
//         .await?
//         .json()?;
//     let user_before_balance: U128 = ft_contract
//         .call( "ft_balance_of")
//         .args_json(json!({"account_id": user.id()}))
//         .transact()
//         .await?
//         .json()?;
//
//     let result = owner
//         .call( ft_contract.id(), "ft_transfer_call")
//         .args_json(serde_json::json!({
//             "receiver_id": user.id(),
//             "amount": amount,
//             "msg": "no parsey as integer big panic oh no",
//         }))
//         .deposit(1)
//         .gas(parse_gas!("200 Tgas") as u64)
//         .transact()
//         .await?;
//     let data: Value = result.json()?;
//     println!("test_transfer_call_promise_panics_for_a_full_refund: {:?}", data);
//     Ok(())
// }
