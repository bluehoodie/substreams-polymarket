.PHONY: build-exchange build-ctf build-neg-risk-ctf build-neg-risk-adapter build-collateral build-wallet-factory build-uma-oracle build-all

build-exchange:
	cd polymarket-exchange && substreams build

build-ctf:
	cd polymarket-ctf && substreams build

build-neg-risk-ctf:
	cd polymarket-neg-risk-ctf && substreams build

build-neg-risk-adapter:
	cd polymarket-neg-risk-adapter && substreams build

build-collateral:
	cd polymarket-collateral && substreams build

build-wallet-factory:
	cd polymarket-wallet-factory && substreams build

build-uma-oracle:
	cd polymarket-uma-oracle && substreams build

build-all: build-exchange build-ctf build-neg-risk-ctf build-neg-risk-adapter build-collateral build-wallet-factory build-uma-oracle
