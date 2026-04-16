# Aptos Node Audit Scope

> Tracking document — subject to change.

## Bounty Target

- **Repo**: `https://github.com/aptos-labs/aptos-core/tree/mainnet`
- **Platform**: HackenProof
- **Max Reward**: $1,000,000 (Critical)
- **Languages**: Rust (node focus)
- **Files in scope**: 11
- **Total SLOC**: ~3,000

---

## In Scope

### Node Core

| Directory | Component |
|-----------|-----------|
| `aptos-node/` | Entry point, wires all subsystems |
| `consensus/` | BFT consensus (excluding `consensus/src/dag/`) |
| `consensus/safety-rules/` | Safety-critical voting/signing rules |
| `execution/` | Transaction execution orchestration |
| `mempool/` | Transaction validation and ordering |
| `network/` | P2P networking, Noise IK, peer management |
| `state-sync/` | State synchronization, data verification |
| `storage/` | RocksDB, JellyfishMerkleTree, pruning |
| `api/` | REST API — external attack surface |
| `config/` | Node config parsing, sanitization, identity loading |
| `types/` | Core type definitions |

### Security-Critical Crates

| Crate | Reason |
|-------|--------|
| `crates/aptos-crypto/` | BLS, x25519, Ed25519, hashing |
| `crates/aptos-node-identity/` | Node identity management |
| `crates/aptos-infallible/` | Lock wrappers (panic on poison) |
| `crates/channel/` | Internal message channels |
| `crates/bounded-executor/` | Task scheduling bounds — DoS relevant |
| `crates/reliable-broadcast/` | Reliable message broadcast |
| `crates/validator-transaction-pool/` | Validator-specific tx pool |
| `crates/aptos-jwk-consensus/` | JWK consensus for keyless |
| `crates/aptos-telemetry/` | Telemetry — info leakage surface |
| `crates/aptos-inspection-service/` | Debug endpoints — info leakage |

### Node-Adjacent

| Directory | Reason |
|-----------|--------|
| `secure/` | Secure storage and secure net |
| `dkg/` | Distributed key generation runtime |
| `peer-monitoring-service/` | Peer health monitoring |

---

## Out of Scope (per bounty)

| Target | Reason |
|--------|--------|
| `consensus/src/dag/` | Explicitly excluded |
| `experimental/` | Explicitly excluded |
| `keyless/pepper/` | Explicitly excluded |
| `[AIP-103] Permissioned Signer` | Explicitly excluded |
| `[AIP-104] Account Abstraction` | Explicitly excluded |

## Excluded (not relevant to node audit)

| Directory | Reason |
|-----------|--------|
| `terraform/`, `docker/`, `scripts/` | Infrastructure |
| `ecosystem/`, `sdk/` | Client-side tooling |
| `tools/` | CLI/debugging tools |
| `dashboards/` | Grafana dashboards |
| `testsuite/` | Test infrastructure |
