DEX
===================

Example implementation of a [Fungible Token] contract which uses [near-contract-standards] and [simulation] tests. And Simple contract with following methods:
* Initialization method: Input is the address of the contract owner and the addresses of two tokens (hereinafter token A and token B).
* The method requests and stores the metadata of tokens (name, decimals)
* Creates wallets for tokens А & В
* The method for getting information about the contract
* Deposit method: The user can transfer a certain number of tokens A to the contract account and in return must receive a certain number of tokens B (similarly in the other direction). The contract supports a certain ratio of tokens A and B. X * Y = K (K is some constant value, X and Y are the number of tokens A and B respectively.
* The owner of the contract can transfer a certain amount of tokens A or B to the contract account, thereby changing the ratio K.
  Implementation requirements in order of their priority.


Requirements
=============

If you're using Gitpod, you can skip this step.

1. Make sure Rust is installed per the prerequisites in [`near-sdk-rs`](https://github.com/near/near-sdk-rs#pre-requisites)
2. Ensure `near-cli` is installed by running `near --version`. If not installed, install with: `npm install -g near-cli`

## Building

To build run:
```bash
make build
```

Using this contract
===================

### Quickest deploy

You can build and deploy this smart contract to a development account. [Dev Accounts](https://docs.near.org/concepts/basics/account#dev-accounts) are auto-generated accounts to assist in developing and testing smart contracts. Please see the [Standard deploy](#standard-deploy) section for creating a more personalized account to deploy to.

```bash
near dev-deploy --wasmFile res/fungible_token.wasm --helperUrl https://near-contract-helper.onrender.com
```

Behind the scenes, this is creating an account and deploying a contract to it. On the console, notice a message like:

>Done deploying to dev-1234567890123

In this instance, the account is `dev-1234567890123`. A file has been created containing a key pair to
the account, located at `neardev/dev-account`. To make the next few steps easier, we're going to set an
environment variable containing this development account id and use that when copy/pasting commands.
Run this command to the environment variable:

```bash
source neardev/dev-account.env
```

You can tell if the environment variable is set correctly if your command line prints the account name after this command:
```bash
echo $CONTRACT_NAME
```

The next command will initialize the contract using the `new` method:

```bash
near call $CONTRACT_NAME new '{"owner_id": "'$CONTRACT_NAME'", "total_supply": "1000000000000000", "metadata": { "spec": "ft-1.0.0", "name": "Token Name A", "symbol": "TokenA", "decimals": 8}' --accountId $CONTRACT_NAME
```

To get the fungible token metadata:

```bash
near view $CONTRACT_NAME ft_metadata
```

Deploying the `TokenB`  and `Defi` contracts are same as above steps.

Transfer Example
---------------

Let's set up an account to transfer some tokens to. These account will be a sub-account of the NEAR account you logged in with.

    near create-account bob.$ID --masterAccount $ID --initialBalance 1

Add storage deposit for Bob's account:

    near call $ID storage_deposit '' --accountId bob.$ID --amount 0.00125


Check balance of Bob's account, it should be `0` for now:

    near view $ID ft_balance_of '{"account_id": "'bob.$ID'"}'

Transfer tokens to Bob from the contract that minted these fungible tokens, exactly 1 yoctoNEAR of deposit should be attached:

    near call $ID ft_transfer '{"receiver_id": "'bob.$ID'", "amount": "19"}' --accountId $ID --amount 0.000000000000000000000001


Check the balance of Bob again with the command from before and it will now return `19`.

## Testing
`make test`
