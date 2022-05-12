#!/bin/bash

BIN=../target/x86_64-unknown-linux-musl/release/aa_cgi
REMOTE=dan@d2718.net:~/wr/cgi-bin/aa_cgi.cgi

cargo build --release --target x86_64-unknown-linux-musl
strip $BIN
scp $BIN $REMOTE

