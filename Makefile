all: clippy test i3mac

i3mac:
	cargo build --release --color auto

clippy:
	cargo clippy --workspace --color auto

test:
	cargo test --workspace --color auto

clean:
	cargo clean

distclean:
	$(RM) -r .cargo

run:
	./target/release/main --padding 50

.PHONY: clippy clean distclean run
