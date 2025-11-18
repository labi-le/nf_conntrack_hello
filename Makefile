.PHONY: build-aarch64 kill-process push-router push-service

push-router: build-aarch64 kill-process
	scp target/aarch64-unknown-linux-musl/release/nf_conntrack_hello  \
		router:/usr/sbin/nf_conntrack_hello

kill-process:
	ssh router 'killall -2 nf_conntrack_hello >/dev/null 2>&1 || true'

build-aarch64:
	cargo build --release --target aarch64-unknown-linux-musl

push-service:
	scp service router:/etc/init.d/nf_conntrack_hello && \
	 ssh router 'chmod +x /etc/init.d/nf_conntrack_hello'