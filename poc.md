# Aptos Node Security Audit — PoC Guide

## Overview

This is a security audit of the **Aptos Network** node codebase (`aptos-core`, mainnet branch).
The audit targets the Rust node implementation and Move framework modules.
Bug bounty hosted on **HackenProof** with rewards up to **$1,000,000** for critical findings.

## PoC Templates

### Node-Level PoC (primary)

Tests the node itself — config, network setup, service initialization.

```
aptos-node/src/poc.rs
```

```bash
cargo test -p aptos-node --lib -- poc::test_poc --nocapture
```

**Helpers available:**
- `default_validator_config()` — lightweight config, no keys (for config-only tests)
- `full_test_config()` — full genesis + keys + identity (for network/identity tests)
- `mock_event_service()` — mock event subscription service
- `setup_test_networks(config, event_service)` — set up node networks (needs full_test_config)
- `config_with_override(yaml)` — merge YAML override into default config

### Move-Level PoC (supplementary)

Tests Move framework interactions — staking, transfers, delegation pools, governance.

```
aptos-move/e2e-move-tests/src/tests/poc.rs
```

```bash
cargo test -p e2e-move-tests -- poc::test_poc --nocapture
```

### How They Work

Both templates run entirely locally — no RPC or network connection needed.

The **node-level** template uses `NodeConfig` and the node's internal modules to test config parsing, network setup, and service wiring.

The **Move-level** template uses `MoveHarness` to simulate the full blockchain environment including account creation, transaction execution, epoch advancement, and state reads.

### Template Helpers Available

| Helper | Purpose |
|--------|---------|
| `get_apt_balance(h, addr)` | Read APT balance (CoinStore + FungibleStore) |
| `get_block_time_secs(h)` | Current block timestamp |
| `transfer_apt(h, sender, receiver, amount)` | Transfer APT |
| `create_account(h, sender, addr)` | Create new account |
| `initialize_staking(h, owner, amount, operator, voter)` | Initialize stake pool |
| `setup_staking(h, owner, amount)` | Initialize + rotate key + join validator set |
| `rotate_consensus_key(h, operator, pool)` | Rotate BLS consensus key |
| `join_validator_set(h, operator, pool)` | Join validator set |
| `leave_validator_set(h, operator, pool)` | Leave validator set |
| `unlock_stake(h, owner, amount)` | Unlock stake |
| `withdraw_stake(h, owner, amount)` | Withdraw unlocked stake |
| `get_stake_pool(h, addr)` | Read StakePool resource |
| `get_validator_set(h)` | Read ValidatorSet |
| `get_validator_config(h, addr)` | Read ValidatorConfig |
| `initialize_delegation_pool(h, owner, commission, seed)` | Create delegation pool |
| `get_delegation_pool_address(h, owner)` | Get pool address via view function |
| `delegation_pool_add_stake(h, delegator, pool, amount)` | Add stake to delegation pool |
| `delegation_pool_unlock(h, delegator, pool, amount)` | Unlock from delegation pool |
| `delegation_pool_withdraw(h, delegator, pool, amount)` | Withdraw from delegation pool |
| `create_staking_contract(h, staker, operator, voter, amount, commission)` | Create staking contract |
| `create_governance_proposal(h, proposer, pool, hash, loc, hash)` | Create governance proposal |
| `vote_on_proposal(h, voter, pool, id, should_pass)` | Vote on proposal |
| `call_entry_function(h, account, "0x1::module::func", ty_args, args)` | Call any entry function |
| `call_view_function(h, "0x1::module::func", ty_args, args)` | Call any view function |

### Key Patterns

```rust
// Create accounts
let attacker = h.new_account_at(AccountAddress::from_hex_literal("0xdead").unwrap());

// Fund with custom amount (default is 10M APT)
let account = h.new_account_with_balance_at(addr, 500_000_000_000); // 5000 APT

// Advance time
h.fast_forward(7200); // 7200 seconds
h.new_epoch();         // fast_forward(7200) + new_block

// Simulate block proposals
h.new_block_with_metadata(proposer_addr, vec![failed_proposer_indices]);

// BCS encode arguments
vec![bcs::to_bytes(&addr).unwrap(), bcs::to_bytes(&amount_u64).unwrap()]

// Assert transaction success/failure
assert_success!(status);
assert_abort!(status, error_code);
```

### Gotchas

- **BCS deserialization**: Does not support `serde::de::IgnoredAny`. Use exact field types or view functions.
- **View functions**: Not all Move functions have `#[view]`. Check the source before calling.
- **Delegation pools**: `MIN_COINS_ON_SHARES_POOL = 1_000_000_000` octas (10 APT minimum).
- **Pool addresses**: Use `get_delegation_pool_address()` view function, not manual seed derivation.
- **Staking contract pool addresses**: Use `default_stake_pool_address(staker, operator)`.
- **Lockup duration**: 7200 seconds in test genesis. `increase_lockup` fails if lockup already at max.
- **Epoch duration**: `new_epoch()` advances 7200 seconds and creates a new block.
- **Custom resources**: Prefer built-in Rust types (`StakePool`, `ValidatorSet`, `ValidatorConfig`) over hand-rolled BCS deserialization.

## Bounty Program Rules

- KYC and PoC are required
- $5 submission fee
- All testing must be done locally — never interact with mainnet/testnet/devnet
- Report vulnerabilities within 24 hours of discovery
- Do not disclose vulnerabilities outside the program without written consent
- Duplicate reports share a single bounty pot — first reporter gets priority

### Reward Tiers

| Severity | Max Reward |
|----------|------------|
| Critical | $1,000,000 |
| High     | $50,000    |
| Medium   | $10,000    |

### Vulnerabilities of Interest

- Loss of Funds (Theft or Minting)
- Consensus / Safety Violations
- Non-recoverable network partition (fix requires hardfork)
- Total Loss of Liveness / Network Availability
- Permanent freezing of funds (fix requires hardfork)
- Remote Code Execution on Validator Node
- Cryptographic Vulnerabilities (with proven impact)
- Validator Node Slowdowns
- API Crash
- DoS issues fixable without hardfork (Medium severity)

### Out of Scope

- `consensus/src/dag/`
- `experimental/`
- `keyless/pepper/`
- `[AIP-103] Permissioned Signer`
- `[AIP-104] Account Abstraction`
- Test / Build Infrastructure Attacks
- Social engineering
- Network DoS attacks

## Scope Reference

See `nexus.scope.md` for full file path list.
See `scope.md` for directory-level scope breakdown.
See `APTOS_NODE_AUDIT_NOTES.md` for architecture and security notes.
