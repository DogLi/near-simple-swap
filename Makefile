
build: build_defi build_token

build_token:
	cd contracts/token && \
	cargo build --all --target wasm32-unknown-unknown --release; \
	cp ./target/wasm32-unknown-unknown/release/*.wasm ../../res/; \
	cd -

build_defi:
	cd contracts/defi && \
	cargo build --all --target wasm32-unknown-unknown --release; \
	cp ./target/wasm32-unknown-unknown/release/*.wasm ../../res/; \
	cd -

test:
	cd ./integration-tests; \
	cargo run

lint:
	cd contracts/token && cargo fmt && cargo clippy --target wasm32-unknown-unknown && cd -;\
	cd contracts/defi && cargo fmt && cargo clippy --target wasm32-unknown-unknown;


clean:
	rm -f ./res/*
