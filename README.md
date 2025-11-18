# nf_conntrack_hello

Small daemon that periodically reads `conntrack -L -p tcp`, looks for connections to `dport=443` with enough client‑to‑server traffic and small server‑to‑client response, and adds their `dst` IPs to a given nftables set

Usage:
```shell
nf_conntrack_hello <nft_set_name>
```

### Build and push to router
```shell
make push-router
```

##### init.d service
```shell
make push-service
```