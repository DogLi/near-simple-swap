/*!
Some hypothetical DeFi contract that will do smart things with the transferred tokens
*/
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::fungible_token::metadata::ext_ft_metadata;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_contract_standards::storage_management::StorageBalance;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{
    env, ext_contract, log, near_bindgen, serde, AccountId, Balance, Gas, PanicOnDefault,
    PromiseOrValue,
};
use near_sdk::{BorshStorageKey, Promise, PromiseError};
use serde::{Deserialize, Serialize};

pub const TGAS: u64 = 1_000_000_000_000;
const INITIAL_BALANCE: Balance = 250_000_000_000_000_000_000_000;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StoreKey {
    Token,
    Decimals,
}

#[derive(Deserialize, Serialize)]
pub struct TokenConfig {
    address: AccountId,
    ticker: String,
}

#[derive(Deserialize, Serialize, BorshSerialize, BorshDeserialize)]
pub struct TokenInfo {
    contract_address: AccountId,
    name: String,
    symbol: String,
    decimals: u8,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DeFi {
    // owner address
    owner_id: AccountId,
    address_a: AccountId,
    address_b: AccountId,
    ratio: U128,
    // (symbol, token_info) map
    tokens: LookupMap<String, TokenInfo>,
    // (token_address, ticker) map
    tickers: LookupMap<AccountId, String>,
    pending: bool,
}

// Defining cross-contract interface. This allows to create a new promise.
#[ext_contract(ext_other)]
pub trait External {
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;
}

fn create_subaccount(prefix: &str) -> Promise {
    let subaccount_id =
        AccountId::new_unchecked(format!("{}.{}", prefix, env::current_account_id()));
    Promise::new(subaccount_id)
        .create_account()
        .add_full_access_key(env::signer_account_pk())
        .transfer(INITIAL_BALANCE)
}

#[near_bindgen]
impl DeFi {
    #[init]
    pub fn new(owner_id: AccountId, token_a: TokenConfig, token_b: TokenConfig) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let tokens = LookupMap::new(StoreKey::Token);
        let mut tickers = LookupMap::new(StoreKey::Decimals);
        tickers.insert(&token_a.address, &token_a.ticker);
        tickers.insert(&token_b.address, &token_b.ticker);
        create_subaccount("token_a");
        create_subaccount("token_b");
        let address_a = AccountId::new_unchecked(format!("token_a.{}", env::current_account_id()));
        let address_b = AccountId::new_unchecked(format!("token_b.{}", env::current_account_id()));
        Self {
            owner_id,
            tokens,
            tickers,
            address_a,
            address_b,
            ratio: U128(0),
            pending: false,
        }
    }

    #[private]
    pub fn set_token_info(&mut self, token_address: AccountId) {
        let gas = Gas(5 * TGAS);
        // get the token meta data and store the token
        let p1: Promise = ext_ft_metadata::ext(token_address.clone())
            // .with_attached_deposit(1)
            .with_static_gas(gas)
            .ft_metadata();
        let p2 = Self::ext(env::current_account_id())
            .with_static_gas(gas)
            .set_token_info_callback(token_address);
        p1.then(p2);
    }

    pub fn set_token_info_callback(
        &mut self,
        token_address: AccountId,
        #[callback_result] call_result: Result<FungibleTokenMetadata, PromiseError>,
    ) {
        match call_result {
            Err(e) => {
                log!("can't get metadata info: {:?}", e);
            }
            Ok(meta) => {
                let token_info = TokenInfo {
                    symbol: meta.symbol.clone(),
                    name: meta.name.clone(),
                    contract_address: token_address,
                    decimals: meta.decimals,
                };
                self.tokens.insert(&meta.symbol, &token_info);
            }
        }
    }

    pub fn get_token_info(&self, symbol: String) -> Option<TokenInfo> {
        self.tokens.get(&symbol)
    }

    #[inline]
    fn get_contract_address(&self, symbol: &String) -> AccountId {
        let token_info = self.tokens.get(symbol).unwrap();
        token_info.contract_address
    }

    #[inline]
    fn get_token_address(&self, symbol: &str) -> AccountId {
        match symbol {
            "TokenA" => self.address_a.clone(),
            "TokenB" => self.address_b.clone(),
            _ => unreachable!("only support TokenA and TokenB"),
        }
    }

    /// get the how many tokens of owner_id
    /// symbol: TokenA / TokenB
    /// return:
    #[private]
    pub fn get_swap_token(&mut self, symbol: String) -> PromiseOrValue<Balance> {
        let address = match symbol.as_str() {
            "TokenA" => self.address_a.clone(),
            _ => self.address_b.clone(),
        };
        let gas = Gas(5 * TGAS);
        let token_address = self.get_contract_address(&symbol);
        let p = ext_ft_core::ext(token_address)
            // .with_attached_deposit(1)
            .with_static_gas(gas)
            .ft_balance_of(address);
        p.into()
    }

    /// store the token balance
    pub fn update_pool_token_callback(
        &mut self,
        #[callback_result] call_result: Result<Balance, PromiseError>,
    ) -> Option<Balance> {
        match call_result {
            Err(e) => {
                log!("get token failed: {:?}", e);
                None
            }
            Ok(balance) => Some(balance),
        }
    }

    /// 1. user deposit TokenA to self.owner_id account
    /// 2. calculate how many TokenB balances that should return to user
    /// 3. self.owner_id account transfer TokenB to user account
    pub fn swap_token(&mut self, symbol: String, amount: U128) -> PromiseOrValue<bool> {
        let symbol_target = match symbol.as_str() {
            "TokenA" => "TokenB".to_string(),
            "TokenB" => "TokenA".to_string(),
            _ => return PromiseOrValue::Value(false),
        };
        if self.pending {
            log!("pending");
            return PromiseOrValue::Value(false);
        }
        let gas = Gas(5 * TGAS);
        let token_info = self.tokens.get(&symbol).unwrap();
        let token_info_target = self.tokens.get(&symbol_target).unwrap();
        let contract_address = token_info.contract_address;
        let contract_address_target = token_info_target.contract_address;
        let token_address = self.get_token_address(&symbol);
        let token_address_target = self.get_token_address(&symbol_target);

        self.pending = true;

        // calculate how many balance should return to user
        let promise_token_1 = ext_ft_core::ext(contract_address.clone())
            // .with_attached_deposit(1)
            .with_static_gas(gas)
            .ft_balance_of(self.owner_id.clone());
        let promise_token_2 = ext_ft_core::ext(contract_address_target.clone())
            // .with_attached_deposit(1)
            .with_static_gas(gas)
            .ft_balance_of(self.owner_id.clone());

        let promise_user_withdraw_balance = Self::ext(env::current_account_id())
            // .with_attached_deposit(1)
            .with_static_gas(gas)
            .calculate_target_token(amount);

        let promise_withdraw_balance = promise_token_1
            .and(promise_token_2)
            .then(promise_user_withdraw_balance);

        // transfer token to token_address
        let promise_deposit: Promise = ext_ft_core::ext(contract_address.clone())
            .with_attached_deposit(1)
            .with_static_gas(gas)
            .ft_transfer(token_address, amount, None);

        // withdraw from the owner_id
        let promise_swap: Promise = Self::ext(env::current_account_id())
            .with_static_gas(gas)
            .swap_token_withdraw(contract_address_target, env::current_account_id());

        promise_withdraw_balance
            .and(promise_deposit)
            .then(promise_swap)
            .into()
    }

    /// transfer token from `contract_address_target` to `user_account_id`
    pub fn swap_token_withdraw(
        &mut self,
        contract_address_target: AccountId,
        user_account_id: AccountId,
        #[callback_result] withdraw_balance: Result<U128, PromiseError>,
        #[callback_result] deposit_result: Result<(), PromiseError>,
    ) -> bool {
        match (withdraw_balance, deposit_result) {
            (Err(e1), Err(e2)) => {
                log!("calculate x error: {:?}, user deposit error: {:?}", e1, e2);
                // TODO: return token back to user
                self.pending = false;
                false
            }
            (Err(e), Ok(_)) => {
                log!("calculate x error: {:?}, return token to user", e);
                self.pending = false;
                false
            }
            (Ok(_), Err(e)) => {
                log!("user deposit error: {:?}", e);
                self.pending = false;
                false
            }
            (Ok(withdraw_amount), Ok(_)) => {
                log!("swap token");
                // TODO: withdraw token to user account
                // let promise_deposit = ext_ft_core::ext(token_address_target)
                //     .with_attached_deposit(1)
                //     .with_static_gas(gas)
                //     .ft_transfer(user_account_id, amount, None);
                self.pending = false;
                true
            }
        }
    }

    /// if user deposit TokenA, calculate how many TokenB that will send to user
    pub fn calculate_target_token(
        &self,
        user_balance: U128,
        #[callback_result] token: Result<U128, PromiseError>,
        #[callback_result] token_target: Result<U128, PromiseError>,
    ) -> U128 {
        if let (Ok(balance), Ok(balance_target)) = (token, token_target) {
            if user_balance >= balance {
                env::panic_str("too much balance")
            }
            // x = m / n
            let m = (Balance::from(user_balance)).checked_mul(Balance::from(balance_target));
            let n = (Balance::from(balance)).checked_add(Balance::from(user_balance));
            let x = match (m, n) {
                (Some(m), Some(n)) => U128::from(m / n),
                _ => env::panic_str("pool balance is too large"),
            };
            log!(
                "balance: {:?}, balance_target: {:?}, user balance target: {:?}",
                balance,
                balance_target,
                x
            );
            return x;
        }
        env::panic_str("get pool token failed")
    }

    /// withdraw balance to owner id, so that to change the ratio
    #[private]
    pub fn withdraw_token(&self, symbol: String, amount: U128) -> PromiseOrValue<U128> {
        todo!("withdraw token from address_a or address_b")
        // let gas = Gas(5 * TGAS);
        // let token_address = self.get_contract_address(&symbol);
        // let promise_withdraw: Promise = ext_ft_core::ext(token_address.clone())
        //     .with_attached_deposit(1)
        //     .with_static_gas(gas)
        //     .ft_transfer_call(receiver_account, amount, None, "".into());
        // promise_withdraw.into()
    }

    /// deposit token so that to change the ratio
    #[private]
    pub fn deposit_token(&self, symbol: String, amount: U128) -> PromiseOrValue<()> {
        let gas = Gas(5 * TGAS);
        let contract_address = self.get_contract_address(&symbol);
        let token_address = self.get_token_address(&symbol);

        let promise_deposit: Promise = ext_ft_core::ext(contract_address)
            .with_attached_deposit(1)
            .with_static_gas(gas)
            .ft_transfer(token_address, amount, None);
        promise_deposit.into()
    }

    /// get balance ratio
    #[private]
    pub fn get_token_ratio(&self) -> PromiseOrValue<U128> {
        let gas = Gas(5 * TGAS);
        let contract_address_a = self.get_contract_address(&"TokenA".to_string());
        let contract_address_b = self.get_contract_address(&"TokenB".to_string());
        let promise_token_a = ext_ft_core::ext(contract_address_a)
            .with_static_gas(gas)
            .ft_balance_of(self.address_a.clone());
        let promise_token_b = ext_ft_core::ext(contract_address_b)
            .with_static_gas(gas)
            .ft_balance_of(self.address_b.clone());
        let promise_calculate_ratio = Self::ext(env::current_account_id())
            .with_static_gas(gas)
            .do_calculate_ratio();
        promise_token_a
            .and(promise_token_b)
            .then(promise_calculate_ratio)
            .into()
    }

    /// return  BalanceA * BalanceB
    pub fn do_calculate_ratio(
        &self,
        #[callback_result] balance_a: Result<U128, PromiseError>,
        #[callback_result] balance_b: Result<U128, PromiseError>,
    ) -> U128 {
        if let (Ok(balance_a), Ok(balance_b)) = (balance_a, balance_b) {
            if let Some(result) = Balance::from(balance_a).checked_mul(Balance::from(balance_b)) {
                U128::from(result)
            } else {
                env::panic_str("ratio is too large")
            }
        } else {
            env::panic_str("get balance failed")
        }
    }
}
