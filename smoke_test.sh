#!/usr/bin/env bash

## This smoke test will run the expected command (`cargo run -- transactions.csv > accounts.csv`)
## and check that the output looks somewhat like what we're expecting.

cargo run -- transactions.csv > accounts.csv

expected_headers='client,available,held,total,locked'
expected_account_1='1,1.5,0,1.5,false'
expected_account_2='2,2,0,2,false'

while read -r HEADERS; do
    read -r ONE
    read -r TWO
    
    if [ "$HEADERS" == "$expected_headers" ]; then
        echo "headers OK"
    else
        echo "wrong headers: $HEADERS, expected $expected_headers" && exit 1
    fi

    if [ "$ONE" == "$expected_account_1" ] || [ "$ONE" == "$expected_account_2" ]; then
        echo "first account ok"
    else
        echo "invalid first account: $ONE" && exit 1
    fi
    
    if [ "$TWO" == "$expected_account_1" ] || [ "$TWO" == "$expected_account_2" ]; then
        echo "second account ok"
    else
        echo "invalid second account: $TWO" && exit 1
    fi
    
    if [ "$ONE" != "$TWO" ]; then
        echo "all good"
    else
        echo "unexpected different accounts" && exit 1
    fi
    
done < accounts.csv

