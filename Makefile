# TODO: setup cli and vendor

all: rv la

rv:
	cargo arceos build -r -A riscv64 -c configs/riscv64.toml -L off -F axstd/bus-mmio
	rust-objcopy --strip-all -O binary target/riscv64gc-unknown-none-elf/release/starry kernel-rv

la:
	cargo arceos build -r -A loongarch64 -c configs/loongarch64.toml -L off
	cp target/loongarch64-unknown-none/release/starry kernel-la

clean:
	cargo clean
	rm -f kernel-rv kernel-la

.PHONY: rv la clean
