#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

set -x

cargo build-bpf

CLUSTER_URL="http://localhost:8899"
solana config set --url $CLUSTER_URL

keypair="$HOME"/.config/solana/id.json

if [ ! -f "$keypair" ]; then
    echo Generating keypair ...
    solana-keygen new -o "$keypair" --no-passphrase --silent
fi

solana config set --keypair ${keypair}

sleep 1

for i in {1..5}
do
    solana airdrop 1
done

solana deploy target/deploy/reward_pool.so target/deploy/reward_pool-keypair.json

exit 0
