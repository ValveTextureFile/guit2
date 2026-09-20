#!/usr/bin/bash

echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ building"
cargo build 

echo "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ running"
sudo ./target/debug/guit2