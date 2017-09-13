#!/bin/bash
set -o errexit
set -o pipefail
set -o nounset
req() {
    curl -s -f -X "$1" --data-binary "$2" localhost:3000/"$3" || exit 1
}
if [ "$(which curl)" = "" ]; then
    echo "Curl must be installed"
    exit 1
fi
dotcargoreg=
if [ $(id -u) = 0 ]; then
    dotcargoreg=/root/.cargo/registry
else
    dotcargoreg=$HOME/.cargo/registry
fi
badpath="$dotcargoreg/index/github.com-1ecc6299db9ec823/ai/da/aidantestund"
if [ ! -f "$badpath" ]; then
    echo "Sorry, looks like this isn't going to work - try running in docker"
    exit 1
fi
req POST '{"account_name": "aidanhs", "currency": "'"$badpath"'"}' makeaccount
req GET '{"account_name": "aidanhs"}' dumpbalance
req POST '{"account_name": "a", "currency": "USD"}' makeaccount
req POST '{"account_name": "b", "currency": "USD"}' makeaccount
req POST '{"account_name": "a", "amount": 500}' deposit
req POST '{"account_from": "a", "account_to": "b", "amount": 100}' transfer
req GET '{"account_name": "aidanhs"}' dumpbalance
