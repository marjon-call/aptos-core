// PoC Test Template for Aptos Node Security Audit
// =================================================
//
// This template provides a ready-to-use scaffold for writing Proof of Concept
// exploits against the Aptos node and Move framework. It covers:
//
//   - Staking & validator operations (stake, delegation pools, rewards)
//   - Coin & fungible asset operations (transfers, minting, balances)
//   - Governance & voting
//   - Account operations (creation, key rotation, signer capabilities)
//   - Epoch management & time manipulation
//   - Transaction validation & fee handling
//   - Block metadata & reconfiguration
//
// Usage:
//   cargo test -p e2e-move-tests -- poc::test_poc --nocapture
//
// For verbose output:
//   cargo test -p e2e-move-tests -- poc::test_poc --nocapture 2>&1
//
// Gotchas discovered during validation:
//   - BCS does not support serde::de::IgnoredAny — use exact field types or view functions
//   - Not all Move functions are view functions — check #[view] attribute in source
//   - Delegation pool MIN_COINS_ON_SHARES_POOL = 1_000_000_000 (10 APT)
//   - Use view function to derive delegation pool address, not manual seed construction
//   - Custom BCS deserialization of Move resources is fragile — prefer built-in Rust types
//     (StakePool, ValidatorSet, ValidatorConfig) or view functions over hand-rolled structs
//   - Default account balance from new_account_at() is 1_000_000_000_000_000 (10M APT)

#![allow(unused_imports, dead_code, unused_variables)]

use crate::{
    assert_success,
    // Staking helpers (re-exported from crate root via stake.rs)
    get_stake_pool, get_validator_config, get_validator_set,
    initialize_staking, join_validator_set, leave_validator_set,
    rotate_consensus_key, setup_staking, unlock_stake, withdraw_stake,
    MoveHarness,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{bls12381, PrivateKey, Uniform};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::{default_stake_pool_address, AccountAddress},
    account_config::CORE_CODE_ADDRESS,
    on_chain_config::ValidatorSet,
    stake_pool::StakePool,
    transaction::TransactionStatus,
};
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    parser::parse_struct_tag,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ============================================================================
// FRAMEWORK ADDRESSES
// ============================================================================

/// Aptos framework address (0x1) — core modules: stake, coin, account, governance
const FRAMEWORK_ADDR: &str = "0x1";

/// Core code address for reading global resources (ValidatorSet, etc.)
// Use: aptos_types::account_config::CORE_CODE_ADDRESS

// ============================================================================
// COMMON RESOURCE STRUCT TAGS
// ============================================================================
//
// Use these with harness.read_resource::<T>(addr, parse_struct_tag(...))
//
// Staking:
//   "0x1::stake::StakePool"
//   "0x1::stake::ValidatorConfig"
//   "0x1::stake::ValidatorSet"
//   "0x1::staking_contract::Store"
//
// Delegation Pool:
//   "0x1::delegation_pool::DelegationPool"
//
// Coin / Fungible Asset:
//   "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"
//   "0x1::fungible_asset::FungibleStore"
//
// Account:
//   "0x1::account::Account"
//
// Governance:
//   "0x1::aptos_governance::GovernanceConfig"
//   "0x1::voting::VotingForum<0x1::governance_proposal::GovernanceProposal>"
//
// Timestamp:
//   "0x1::timestamp::CurrentTimeMicroseconds"

// ============================================================================
// HELPER: Read common resources
// ============================================================================

/// Read the current block timestamp in seconds
fn get_block_time_secs(harness: &mut MoveHarness) -> u64 {
    harness.executor.get_block_time_seconds()
}

/// Read APT balance for an account (handles both CoinStore and FungibleStore)
fn get_apt_balance(harness: &MoveHarness, addr: &AccountAddress) -> u64 {
    harness.read_aptos_balance(addr)
}

// ============================================================================
// HELPER: Delegation pool operations
// ============================================================================

fn initialize_delegation_pool(
    harness: &mut MoveHarness,
    owner: &Account,
    commission_percentage: u64,
    _delegation_pool_creation_seed: Vec<u8>,
) -> TransactionStatus {
    harness.run_transaction_payload(
        owner,
        aptos_stdlib::delegation_pool_initialize_delegation_pool(
            commission_percentage,
            vec![],
        ),
    )
}

/// Get the delegation pool address for an owner via on-chain view function
fn get_delegation_pool_address(
    harness: &mut MoveHarness,
    owner_addr: &AccountAddress,
) -> AccountAddress {
    let result = call_view_function(
        harness,
        "0x1::delegation_pool::get_owned_pool_address",
        vec![],
        vec![bcs::to_bytes(owner_addr).unwrap()],
    );
    bcs::from_bytes::<AccountAddress>(&result[0]).unwrap()
}

fn delegation_pool_add_stake(
    harness: &mut MoveHarness,
    delegator: &Account,
    pool_address: AccountAddress,
    amount: u64,
) -> TransactionStatus {
    harness.run_transaction_payload(
        delegator,
        aptos_stdlib::delegation_pool_add_stake(pool_address, amount),
    )
}

fn delegation_pool_unlock(
    harness: &mut MoveHarness,
    delegator: &Account,
    pool_address: AccountAddress,
    amount: u64,
) -> TransactionStatus {
    harness.run_transaction_payload(
        delegator,
        aptos_stdlib::delegation_pool_unlock(pool_address, amount),
    )
}

fn delegation_pool_withdraw(
    harness: &mut MoveHarness,
    delegator: &Account,
    pool_address: AccountAddress,
    amount: u64,
) -> TransactionStatus {
    harness.run_transaction_payload(
        delegator,
        aptos_stdlib::delegation_pool_withdraw(pool_address, amount),
    )
}

// ============================================================================
// HELPER: Account & transfer operations
// ============================================================================

fn transfer_apt(
    harness: &mut MoveHarness,
    sender: &Account,
    receiver: AccountAddress,
    amount: u64,
) -> TransactionStatus {
    harness.run_transaction_payload(
        sender,
        aptos_stdlib::aptos_account_transfer(receiver, amount),
    )
}

fn create_account(
    harness: &mut MoveHarness,
    sender: &Account,
    new_address: AccountAddress,
) -> TransactionStatus {
    harness.run_transaction_payload(
        sender,
        aptos_stdlib::aptos_account_create_account(new_address),
    )
}

// ============================================================================
// HELPER: Governance operations
// ============================================================================

fn create_governance_proposal(
    harness: &mut MoveHarness,
    proposer: &Account,
    stake_pool: AccountAddress,
    execution_hash: Vec<u8>,
    metadata_location: Vec<u8>,
    metadata_hash: Vec<u8>,
) -> TransactionStatus {
    harness.run_transaction_payload(
        proposer,
        aptos_stdlib::aptos_governance_create_proposal_v2(
            stake_pool,
            execution_hash,
            metadata_location,
            metadata_hash,
            true,
        ),
    )
}

fn vote_on_proposal(
    harness: &mut MoveHarness,
    voter: &Account,
    stake_pool: AccountAddress,
    proposal_id: u64,
    should_pass: bool,
) -> TransactionStatus {
    harness.run_transaction_payload(
        voter,
        aptos_stdlib::aptos_governance_vote(stake_pool, proposal_id, should_pass),
    )
}

// ============================================================================
// HELPER: Staking contract operations
// ============================================================================

fn create_staking_contract(
    harness: &mut MoveHarness,
    staker: &Account,
    operator: AccountAddress,
    voter: AccountAddress,
    amount: u64,
    commission_percentage: u64,
) -> TransactionStatus {
    harness.run_transaction_payload(
        staker,
        aptos_stdlib::staking_contract_create_staking_contract(
            operator,
            voter,
            amount,
            commission_percentage,
            vec![],
        ),
    )
}

// ============================================================================
// HELPER: Generic entry & view function calls
// ============================================================================

/// Call any entry function by string path, e.g. "0x1::stake::unlock"
fn call_entry_function(
    harness: &mut MoveHarness,
    account: &Account,
    function_id: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) -> TransactionStatus {
    harness.run_entry_function(
        account,
        str::parse(function_id).unwrap(),
        ty_args,
        args,
    )
}

/// Call a view function and return raw BCS bytes
fn call_view_function(
    harness: &mut MoveHarness,
    function_id: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) -> Vec<Vec<u8>> {
    harness
        .execute_view_function(
            str::parse(function_id).unwrap(),
            ty_args,
            args,
        )
        .values
        .unwrap()
}

// ============================================================================
// PoC TEST — Fill in your exploit logic below
// ============================================================================

#[test]
fn test_poc() {
    // --- Initialize test environment ---
    // let mut h = MoveHarness::new();

    // --- Create accounts ---
    // let attacker = h.new_account_at(AccountAddress::from_hex_literal("0xdead").unwrap());
    // let victim = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // --- Setup initial state ---
    // e.g. initialize staking, fund accounts, create pools

    // --- Execute exploit ---
    // e.g. call entry functions, manipulate time, trigger epoch changes

    // --- Verify impact ---
    // e.g. assert stolen funds, broken invariants, unauthorized state changes
}
