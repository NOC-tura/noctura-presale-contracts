# Noctura Presale Smart Contract

Noctura (NOC) is the first shielded privacy layer on Solana, enabling compliant private transactions.

## Program Details

| Field | Value |
|-------|-------|
| **Program ID** | `6nTTJwtDuxjv8C1JMsajYQapmPAGrC3QF1w5nu9LXJvt` |
| **Token Mint** | `B61SyRxF2b8JwSLZHgEUF6rtn6NUikkrK1EMEgP6nhXW` |
| **Network** | Solana Mainnet |
| **Framework** | Anchor v0.32.1 |

## Features

### Presale
- 10-stage presale with progressive pricing ($0.1501 → $0.3499)
- 102.4M NOC allocation (40% of 256M total supply)
- Multi-currency support: SOL, USDT, USDC
- Cross-chain purchases (ETH, BNB) via coordinator
- Referral system with 10% bonus

### Staking
- Tier A: 365 days lock, 128% APR
- Tier B: 182 days lock, 68% APR  
- Tier C: 90 days lock, 34% APR
- Auto-compound functionality
- Flexible unstaking with cooldown

### Security
- Price feeds via Pyth Network
- Admin controls for pause/unpause
- Per-user and per-transaction limits
- Cross-chain purchase verification

## Building

```bash
# Install dependencies
anchor build

# Verify build matches on-chain program
solana-verify build
```

## Verification

This contract is verified on Solana Explorer. You can verify the build yourself:

```bash
solana-verify verify-from-repo \
  --program-id 6nTTJwtDuxjv8C1JMsajYQapmPAGrC3QF1w5nu9LXJvt \
  https://github.com/NOC-tura/noctura-presale-contract \
  --library-name solana_ico_enhanced
```

## Links

- **Website**: [noc-tura.io](https://noc-tura.io)
- **Explorer**: [Solana Explorer](https://explorer.solana.com/address/6nTTJwtDuxjv8C1JMsajYQapmPAGrC3QF1w5nu9LXJvt)
- **Documentation**: [Whitepaper](https://noc-tura.io/whitepaper)

## License

MIT License - see [LICENSE](LICENSE)
