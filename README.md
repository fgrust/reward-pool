# Soldex Test Assignment

## Scenario

A Solana program for staking any SPL tokens. What functions it has to include:
- Create a token pool, name this function as create_reward_pool function. When this function is called, it should transfer a specified amount of SPL token from the calling account to the program.
- Reward LP tokens. Do not implement this function, but write a specification for another colleague. In general, this function should mint LP tokens for a stake.
- The optional function is to claim rewards from the program to the user's wallet.

## Development
---
### Environment Setup
1. Install the lastest Rust stable from https://rustup.rs/
2. Install Solana v1.8.12 or later from https://docs.solana.com/cli/install-solana-cli-tools
3. Install the `libudev` development package for your distribution(`libudev-dev` on Debian-derived distros, `libudev-devel` on Redhat-derived).
### Build
The normal cargo build is available for building programs against your host machine:
```
$ cargo build
```
To build for the Solana BPF target:
```
$ cargo build-bpf
```
### Test
Unit tests can be run with
```bash
$ cargo test        # <-- runs host-based tests
$ cargo test-bpf    # <-- runs BPF program tests
```

### Clippy
```bash
$ cargo clippy
```

### Deploy to Localnet
```bash
$ ./script/deploy.sh
```
