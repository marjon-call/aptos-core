# Aptos Node Architecture & Security Audit Notes

> Summarized from https://aptos.dev/network/nodes — focused on node internals, architecture, and security-relevant details for code audit of `aptos-core` (mainnet branch).

---

## 1. Node Types

The Aptos network has three node types, all running the same `aptos-node` binary with different configuration:

**Validator Nodes**: Participate in consensus. Require a minimum stake (1 million APT). A validator-leader proposes blocks and earns rewards on success. Validators connect to each other over the Validator network (port `6180`). Validator set changes happen at epoch boundaries (every 2 hours on mainnet).

**Validator Full Nodes (VFNs)**: Run only by validator operators. Connect privately to their paired validator on the VFN network (port `6181`) and serve the public network (port `6182`) for PFNs. Per AIP-139, VFNs are being deprecated — validators can now accept direct PFN connections via a `full_node_networks` config section or default port `6182`. The config flag `base.enable_validator_pfn_connections: false` can reject PFN connections entirely.

**Public Full Nodes (PFNs)**: Can be run by anyone. Connect to VFNs and other PFNs on the public network (port `6182`). Replicate the entire blockchain state but do not participate in consensus. Default data directory: `/opt/aptos/data`.

---

## 2. Node Architecture & Core Transaction Flow

1. **API Layer** (`api/`) — REST API on port `8080` receives transactions
2. **Mempool** (`mempool/`) — Transaction validation and ordering
3. **Consensus** (`consensus/`) — BFT ordering (validators only)
4. **Execution** (`execution/`) — Orchestrates VM execution via Block-STM parallel execution
5. **Move VM** (`third_party/move/`) — Executes Move bytecode
6. **Storage** (`storage/`) — Persistent state using JellyfishMerkleTree, backed by RocksDB
7. **State Sync** (`state-sync/`) — Blockchain synchronization for catching up

### Key Configuration Files

| File | Purpose |
|------|---------|
| `validator.yaml` | Validator node config |
| `fullnode.yaml` | VFN/PFN config |
| `validator-identity.yaml` | Validator keys and account address |
| `validator-full-node-identity.yaml` | VFN keys and account address |
| `private-keys.yaml` | All private keys |
| `public-keys.yaml` | All public keys and account address |
| `genesis.blob` | Genesis block data (from `aptos-networks` repo) |
| `waypoint.txt` | Verifiable checkpoint for genesis |
| `secure-data.json` / `secure_storage.json` | Consensus private keys (**SECURITY CRITICAL**) |

### Important Config Parameters (validator.yaml)

- `base.data_dir` — Blockchain data directory
- `base.waypoint` — Genesis waypoint
- `base.role` — `"validator"` or `"full_node"`
- `consensus.safety_rules.initial_safety_rules_config.from_file.identity_blob_path` — Path to validator-identity.yaml
- `consensus.safety_rules.initial_safety_rules_config.from_file.overriding_identity_paths` — Additional consensus identity files (used for key rotation)
- `execution.genesis_file_location` — Path to genesis.blob
- `storage.rocksdb_configs.enable_storage_sharding` — Should be `true`
- `validator_network.identity` — Validator network identity config

---

## 3. Consensus

Aptos uses a BFT consensus protocol where a validator-leader proposes blocks.

- **Epoch duration**: 2 hours on mainnet
- **Validator set updates**: Happen at epoch boundaries only
- **Leader election**: Based on stake and reputation
- **Rewards**: Based on proposal success rate (successful proposals / total proposals). Rewards only for proposing, not voting. Distributed at end of epoch.
- **Consensus key**: BLS key with proof of possession, stored in `secure_storage.json`

### Consensus Observer (AIP-93)

A data dissemination technique for fullnodes (VFNs and PFNs) that reduces block sync time and transaction latencies by 10-50%.

```yaml
consensus:
  enable_pre_commit: false  # Set to true to DISABLE consensus observer
consensus_observer:
  observer_enabled: true
  publisher_enabled: true
```

### Safety Rules

The `consensus/safety-rules/` directory is explicitly called out as **safety-critical code**. The safety rules config in `validator.yaml` references `validator-identity.yaml` and includes waypoint verification.

---

## 4. State Synchronization

State sync has two phases: bootstrapping and continuous syncing.

### Bootstrapping Modes

| Mode | Security | Speed |
|------|----------|-------|
| `ExecuteTransactionsFromGenesis` | Most secure — re-executes all transactions | Slowest |
| `ApplyTransactionOutputsFromGenesis` | Trusts validators for execution correctness | Faster |
| `ExecuteOrApplyFromGenesis` | Hybrid intelligent syncing, adapts per chunk | Balanced |
| `DownloadLatestStates` | Fast sync — skips history, downloads latest state | Fastest |

### Continuous Syncing Modes

- `ExecuteTransactions` — Re-executes each transaction
- `ApplyTransactionOutputs` — Applies outputs directly
- `ExecuteTransactionsOrApplyOutputs` — Intelligent hybrid

```yaml
state_sync:
  state_sync_driver:
    bootstrapping_mode: DownloadLatestStates
    continuous_syncing_mode: ExecuteTransactionsOrApplyOutputs
```

### Trust Model

- All modes verify cryptographic signatures from validators over blockchain data.
- Fast sync requires trusting validators for historical transaction execution correctness.
- Execute-from-genesis is the most secure mode, verifying both consensus agreement and execution results.
- Root of trust: Validator set + cryptographic signatures over blockchain data.

---

## 5. Networking

### Three Network Types

| Network | Port | Purpose |
|---------|------|---------|
| Validator Network | `6180` | Validator-to-validator only |
| VFN Network | `6181` | Private link between validator and its VFN |
| Public Network | `6182` | VFNs and PFNs connect to each other |

### Transport Protocol

Uses **Noise IK protocol** for authenticated encryption. Multiaddr format:

```
/ip4/<IP>/tcp/<Port>/noise-ik/<Public_Key>/handshake/0
/dns4/<DNS>/tcp/<Port>/noise-ik/<Public_Key>/handshake/0
```

Uses **x25519** key pairs for network identity.

### Network Identity

- PFNs get a randomly generated (ephemeral) identity by default, stored at `/opt/aptos/data/db/ephemeral_identity_key`
- Static identities can be configured using `aptos key generate --key-type x25519`
- Identity is configured in YAML with `type: "from_config"`, `key`, and `peer_id`

### Peer Discovery

- `discovery_method: "onchain"` — Discovers peers from on-chain validator set data
- `discovery_method: "none"` — No automatic discovery
- Manual `seeds` configuration for specific peer connections
- Peer roles: `Upstream`, `Downstream`, `PreferredUpstream`

### Private PFN Configuration

```yaml
full_node_networks:
  - discovery_method: "onchain"
    max_inbound_connections: 0
    mutual_authentication: true
```

---

## 6. Storage & Data

### Storage Backend

RocksDB with optional sharding (`storage.rocksdb_configs.enable_storage_sharding: true`).

### Data Pruning

Ledger history is pruned by default:

```yaml
storage:
  storage_pruner_config:
    ledger_pruner_config:
      enable: true/false
      prune_window: 100000000  # Number of recent transactions to retain
```

Archival nodes must disable the pruner and must NOT use fast sync.

### Backup Data Structure (from `aptos-db-restore`)

- `epoch_ending` — LedgerInfo at end of each epoch, proves epoch provenance from genesis
- `state_snapshot` — State Merkle Tree (SMT) snapshot at a specific version
- `transaction` — Raw transaction metadata, payload, VM outputs, and cryptographic proofs

Public backups on AWS S3 and GCS, backed by cryptographic proofs. Restore tool: `aptos node bootstrap-db`.

### Bootstrap Methods

1. State sync (fast sync mode) — Default
2. Snapshot download from community providers
3. Database restore from backup using `aptos node bootstrap-db`

Full history restore requires: `ulimit -n 1048576`, 32GB RAM, 1-1.5TB disk.

---

## 7. Staking & Validation

### Two Pool Types (cannot be changed once created)

**Staking Pool**: Only accepts stake from pool owner. Requires 1 million APT minimum.

**Delegation Pool**: Accepts stake from multiple delegators. Minimum 10 APT per delegator. Uses `0x1::delegation_pool` module. Resource account created with `owner_address` + `delegation_pool_creation_seed`.

### Key Roles (separate accounts recommended)

| Role | Purpose |
|------|---------|
| Owner | Controls the staking pool, assigns operator and voter |
| Operator | Runs the node, receives commission |
| Voter | Participates in governance votes |
| Beneficiary | Can receive operator commission (optional) |

### Commission

Set at initialization for delegation pools (e.g., `u64:1000` = 10%). Can be updated with 7.5 day notice before lockup cycle ends. Must call `synchronize_delegation_pool` after lockup ends.

### Validator States

`Active` → `Pending_inactive` → `Inactive` → `Pending_active` → `Active`

State transitions occur at epoch boundaries.

### Delegation Pool Allowlisting

Pool owners can enable/disable permissioned access, evict non-allowlisted delegators (unlocks entire stake). Evicted delegators can still withdraw after lockup. Re-allowlisted delegators must manually call `reactivate_stake`.

### Key On-Chain Functions

- `0x1::stake::rotate_consensus_key`
- `0x1::delegation_pool::initialize_delegation_pool`
- `0x1::delegation_pool::add_stake`, `unlock`, `withdraw`, `reactivate_stake`
- `0x1::delegation_pool::set_operator`, `set_beneficiary_for_operator`
- `0x1::delegation_pool::enable_delegators_allowlisting`, `evict_delegator`
- `0x1::staking_contract::update_commission`

---

## 8. Security-Relevant Details

### Cryptographic Keys

| Key Type | Algorithm | Storage | Purpose |
|----------|-----------|---------|---------|
| Consensus key | BLS (with proof of possession) | `secure_storage.json` | Block proposals and voting |
| Network identity key | x25519 | `validator-identity.yaml` or ephemeral | Noise IK handshake |
| Account private key | Ed25519 | `private-keys.yaml` | Operator account operations |

### Key Rotation

- **Consensus key rotation**: Add new key to `overriding_identity_paths` in config, restart node, then call `0x1::stake::rotate_consensus_key` on-chain with new public key and proof of possession. Takes effect at next epoch (2 hours).
- **Old keys are NOT automatically deleted from `secure_storage.json`** — manual cleanup required.
- Network address updates also require on-chain transaction.

### Identity Files (SECURITY CRITICAL)

Files to preserve for node identity:
- `public-keys.yaml`
- `private-keys.yaml`
- `validator-identity.yaml`
- `validator-full-node-identity.yaml`

### Port Security

| Port | Service | Validator | VFN | PFN |
|------|---------|-----------|-----|-----|
| 6180 | Validator network | OPEN | — | — |
| 6181 | VFN network | OPEN | OPEN | — |
| 6182 | Public network | CLOSE | OPEN | OPEN |
| 8080 | REST API | CLOSE | CLOSE | CLOSE |
| 9101 | Inspection | CLOSE | CLOSE | CLOSE |
| 9102 | Admin | CLOSE | CLOSE | CLOSE |

### Authentication

- `mutual_authentication: true` in network config requires authenticated connections (Noise IK)
- `max_inbound_connections: 0` prevents unauthenticated inbound connections (private PFN mode)
- Validators only accept connections from known peers in the validator set

### Inspection Service (port 9101)

```yaml
inspection_service:
  port: 9101
  expose_configuration: true/false
  expose_system_information: true/false
  expose_identity_information: true/false
```

Endpoints: `/configuration`, `/consensus_health_check`, `/forge_metrics`, `/identity_information`, `/json_metrics`, `/metrics`, `/peer_information`, `/system_information`

### Telemetry Privacy Concerns

By default, nodes send telemetry to Aptos Labs including:
- Core metrics (state sync, consensus, mempool, storage)
- Build information, system information (CPU, RAM, disk, network, OS)
- Network metrics (connected peers, messages)
- Prometheus metrics (all runtime metrics)
- Warn-level and higher logs
- `APTOS_DISABLE_LOG_ENV_POLLING` — When NOT disabled, allows dynamic verbose log sending (potential information leakage surface)

Disable with `APTOS_DISABLE_TELEMETRY=true`.

### Kubernetes Secrets (Cloud Deployments)

Identity files, genesis.blob, and waypoint.txt stored as K8s secrets.

---

## 9. Metrics & Health

### Key Metrics (port 9101)

**Consensus** (validators only):
- `aptos_consensus_proposals_count` — Block proposals sent
- `aptos_consensus_last_committed_round` — Last committed round (should increase multiple times/sec)
- `aptos_consensus_timeout_count` — Local timeout count (increases = problems)

**State sync**:
- `aptos_state_sync_version{type="synced"}` — Current synced version
- `aptos_data_client_highest_advertised_data{data_type="transactions"}` — Highest version advertised by peers

**Network**:
- `aptos_connections{direction="inbound"}` / `aptos_connections{direction="outbound"}`

**Mempool**:
- `core_mempool_index_size{index="system_ttl"}` — Transactions waiting

**REST API**:
- `aptos_api_requests_count{method="GET/POST"}`
- `aptos_api_response_status_count`

### Node Health Checker (NHC)

Runs as a separate service (port `20121`). Compares target node against a baseline. Checks: chain ID, role type, transaction availability, latency (max 1000ms for 2 round trips). Crate: `aptos-node-checker`.

### Hardware Requirements (Mainnet)

- CPU: 48 threads, 5th Gen AMD EPYC or 6th Gen Intel Xeon
- Memory: 128GB RAM
- Storage: 3.0 TB Enterprise NVMe SSD, 60K IOPS, 600MiB/s
- Network: 1Gbps
- Target throughput: ~30,000 TPS

---

## 10. Key Source Code Directories for Node Audit

| Directory | Component | Notes |
|-----------|-----------|-------|
| `consensus/` | BFT consensus | Core consensus logic |
| `consensus/safety-rules/` | Safety rules | **Safety-critical** |
| `execution/` | Transaction execution | Block-STM parallel execution |
| `mempool/` | Transaction pool | Validation and ordering |
| `storage/` | Persistent state | RocksDB, JellyfishMerkleTree |
| `state-sync/` | State synchronization | Bootstrapping and continuous sync |
| `network/` | P2P networking | Noise IK, peer discovery |
| `api/` | REST API | Transaction submission |
| `aptos-move/block-executor/` | Block-STM | Parallel execution engine |
| `aptos-move/framework/` | Move framework | Core chain modules (coin, account, staking) |
| `aptos-move/framework/aptos-framework/` | Aptos framework | `stake`, `delegation_pool`, `account` |
| `third_party/move/` | Move VM | Bytecode execution, compilation, verification |
| `crates/aptos-crypto/` | Cryptographic primitives | **Security-critical** |
| `secure/` | Security modules | **Security-critical** |
| `keyless/` | Keyless authentication | **Security-critical** (but `keyless/pepper` is out of scope) |
| `types/` | Core types | Used everywhere |

### Out of Scope (per bounty)

- `consensus/src/dag/`
- `experimental/`
- `keyless/pepper/`
- `[AIP-103] Permissioned Signer`
- `[AIP-104] Account Abstraction`
