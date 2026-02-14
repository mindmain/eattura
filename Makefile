dev:
	buf export buf.build/bufbuild/protovalidate --output proto/vendor
	cargo run --bin eattura

build:
	buf export buf.build/bufbuild/protovalidate --output proto/vendor
	cargo build --release