.PHONY: build-exchange build-ctf build-neg-risk-ctf build-all

build-exchange:
	cd polymarket-exchange && substreams build

build-ctf:
	cd polymarket-ctf && substreams build

build-neg-risk-ctf:
	cd polymarket-neg-risk-ctf && substreams build

build-all: build-exchange build-ctf build-neg-risk-ctf
