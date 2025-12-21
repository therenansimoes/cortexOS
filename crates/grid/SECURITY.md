# Grid Security Documentation

This document describes the security features and protocols implemented in the CortexOS Grid networking layer.

## Overview

The Grid handshake protocol provides secure peer-to-peer communication with end-to-end encryption, authentication, and protection against various attack vectors.

## Security Features

### 1. End-to-End Encryption (E2E)

**Key Exchange**: X25519 Diffie-Hellman
- Ephemeral key pairs generated for each handshake
- Public keys exchanged during HELLO and CHALLENGE messages
- Shared secret derived using X25519 key agreement

**Symmetric Encryption**: ChaCha20-Poly1305
- Session keys derived from shared secret using BLAKE3 KDF
- Context string: `"cortex-session-v1"`
- Authenticated encryption with associated data (AEAD)
- 12-byte random nonces for each encrypted message
- Post-handshake messages encrypted with session keys

### 2. Authentication

**Signature Algorithm**: Ed25519
- Each node has a long-term Ed25519 signing key
- Node IDs derived from Ed25519 public keys using BLAKE3
- All handshake messages signed with Ed25519

**Challenge-Response Protocol**:
1. Responder generates random 32-byte nonce
2. Initiator signs nonce with its Ed25519 key
3. Responder verifies signature to prove liveness
4. Prevents replay attacks and ensures bidirectional authentication

### 3. MITM Protection

**Public Key Verification**:
- Node IDs are derived from Ed25519 public keys
- Each message includes node_id and pubkey
- Verifier checks: `BLAKE3(pubkey) == node_id`
- Prevents impersonation attacks

**Signature Verification**:
- HELLO message includes signed data covering:
  - Protocol version
  - Node ID
  - Ed25519 public key
  - Capabilities
  - X25519 public key
  - Timestamp
- Signature verified before processing message

### 4. Replay Attack Prevention

**Timestamp Validation**:
- Maximum allowed time drift: 5 minutes (300 seconds)
- Prevents replay of old handshake messages
- Checked on HELLO message reception
- Rejects messages outside acceptable time window

**Nonce Tracking**:
- Last 100 nonces stored per handshake context
- Duplicate nonces rejected immediately
- Prevents same-session replay attacks
- Memory-bounded to prevent DoS

### 5. Denial of Service (DoS) Protection

**Handshake Timeout**:
- Target: < 100ms
- Actual performance: ~6ms (median)
- Timeout enforced throughout handshake
- Prevents resource exhaustion from slow peers

**Rate Limiting** (future):
- Per-peer handshake rate limits
- Global handshake rate limits
- Adaptive throttling under load

## Handshake Protocol

### Message Flow

```
Initiator                    Responder
    |                            |
    |  1. HELLO                  |
    |  - protocol_version        |
    |  - node_id                 |
    |  - ed25519_pubkey          |
    |  - capabilities            |
    |  - x25519_pubkey           |
    |  - timestamp               |
    |  - signature               |
    |--------------------------->|
    |                            |
    |  2. CHALLENGE              |
    |  - nonce (32 bytes)        |
    |  - x25519_pubkey           |
    |<---------------------------|
    |                            |
    |  3. PROVE                  |
    |  - signature(nonce)        |
    |--------------------------->|
    |                            |
    |  4. WELCOME                |
    |  - session_id              |
    |  - session_params          |
    |<---------------------------|
    |                            |
   [Session Keys Derived]   [Session Keys Derived]
    |                            |
```

### Session Key Derivation

Both parties derive identical session keys:

```rust
shared_secret = x25519_secret.diffie_hellman(&remote_x25519_public)
encryption_key = BLAKE3::derive_key("cortex-session-v1", shared_secret)
```

Session keys include:
- `session_id`: Random 32-byte identifier
- `encryption_key`: 32-byte key for ChaCha20-Poly1305
- `initiated_at`: Timestamp for key rotation

## Security Considerations

### Threats Mitigated

✅ **Man-in-the-Middle (MITM)**
- Ed25519 signatures prevent impersonation
- Node ID verification ensures pubkey authenticity
- X25519 key exchange provides forward secrecy

✅ **Replay Attacks**
- Timestamp validation (5-minute window)
- Nonce tracking prevents duplicate challenges
- Fresh keys for each session

✅ **Eavesdropping**
- End-to-end encryption with ChaCha20-Poly1305
- Session keys derived from ephemeral X25519 keys
- Perfect forward secrecy

✅ **Denial of Service**
- Handshake timeout enforcement (< 100ms)
- Bounded nonce storage (100 entries)
- Efficient cryptographic primitives

### Potential Vulnerabilities

⚠️ **Clock Skew**
- 5-minute timestamp window may be too permissive
- Consider: Reduce to 1 minute for stricter replay prevention
- Mitigation: NTP synchronization recommended

⚠️ **Key Rotation**
- Session keys persist until connection closes
- Consider: Periodic re-keying for long-lived connections
- Mitigation: Implement key rotation after N messages or T seconds

⚠️ **Amplification Attacks**
- HELLO message may trigger large CHALLENGE response
- Consider: Rate limiting handshake attempts
- Mitigation: Planned for PR #11 (Security Hardening)

## Performance

### Benchmark Results

```
Target latency: < 100ms
Iterations: 100

Results:
  Successes: 100/100
  Min:       5.90ms
  Median:    6.01ms
  Average:   6.01ms
  P95:       6.08ms
  P99:       6.22ms
  Max:       6.22ms

✓ PASS: Well under 100ms target
```

### Cryptographic Operations

Per handshake:
- 2x Ed25519 signature generation (~30µs each)
- 2x Ed25519 signature verification (~60µs each)
- 2x X25519 key generation (~20µs each)
- 1x X25519 shared secret (~20µs)
- 1x BLAKE3 KDF (~5µs)

Total crypto overhead: ~245µs
Remaining time: Network + serialization

## Future Enhancements

### Planned for PR #11 (Relay Mesh Security Hardening)

- [ ] Beacon rate limiting
- [ ] Key rotation mechanism
- [ ] Spam prevention
- [ ] Enhanced DoS protection
- [ ] Security audit integration

### Potential Improvements

- [ ] Post-quantum cryptography option (Kyber + Dilithium)
- [ ] Certificate pinning for known peers
- [ ] Revocation lists for compromised keys
- [ ] Hardware security module (HSM) support
- [ ] Formal verification of protocol

## References

- [X25519 RFC 7748](https://datatracker.ietf.org/doc/html/rfc7748)
- [ChaCha20-Poly1305 RFC 8439](https://datatracker.ietf.org/doc/html/rfc8439)
- [Ed25519 RFC 8032](https://datatracker.ietf.org/doc/html/rfc8032)
- [BLAKE3 Specification](https://github.com/BLAKE3-team/BLAKE3-specs)

## Testing

Security tests located in `src/handshake.rs`:

- `test_handshake_flow` - Complete handshake with session keys
- `test_session_encryption` - E2E encryption roundtrip
- `test_replay_attack_prevention` - Nonce reuse detection
- `test_timestamp_validation` - Old message rejection
- `test_nonce_reuse_detection` - Duplicate nonce handling
- `test_protocol_version_mismatch` - Version checking
- `test_invalid_signature` - Signature verification

Run tests:
```bash
cargo test --package cortex-grid --lib handshake
```

Run benchmarks:
```bash
cargo bench --package cortex-grid --bench handshake_benchmark
```

## Security Contact

For security issues, please follow the guidelines in [SECURITY.md](../../SECURITY.md) in the repository root.
