# Nexus datapump developer tasks.
#
#   make test          run unit tests
#   make clippy        run clippy with warnings denied
#   make check         format-check + tests + clippy
#   make run-mqtt      publish test data to MQTT
#   make run-zenoh     publish test data to Zenoh
#   make mqtt-broker   start a no-auth local MQTT broker
#   make zenoh-broker  start the Nexus no-auth Zenoh test router
#   make smoke-mqtt    start MQTT broker, publish a short batch, stop broker
#   make smoke-zenoh   start Zenoh router, publish a short batch, stop router
#   make clean         remove Rust build output

NEXUS_DIR := $(abspath ../..)

TRANSPORT      ?= mqtt
COUNT          ?=
INTERVAL_MS    ?= 1000
PATH_PREFIX    ?= rubix/testing
TENANT_ID      ?= *
PATH_TENANT    ?= all
SITES          ?= 3
METERS_PER_KIND ?= 4
SEED           ?= 42
RUST_LOG       ?= info

MQTT_CONTAINER ?= nexus-test-mqtt
MQTT_IMAGE     ?= eclipse-mosquitto:2
MQTT_HOST      ?= 127.0.0.1
MQTT_PORT      ?= 1883
MQTT_CLIENT_ID ?= nexus-datapump
MQTT_QOS       ?= 0

ZENOH_ENDPOINT ?= tcp/127.0.0.1:7447

COMMON_ARGS := --path-prefix "$(PATH_PREFIX)" \
	--tenant-id "$(TENANT_ID)" \
	--path-tenant "$(PATH_TENANT)" \
	--sites $(SITES) \
	--meters-per-kind $(METERS_PER_KIND) \
	--interval-ms $(INTERVAL_MS) \
	--seed $(SEED)

ifneq ($(strip $(COUNT)),)
COMMON_ARGS += --count $(COUNT)
endif

.PHONY: help fmt fmt-check test clippy check run run-mqtt run-zenoh \
        mqtt-broker mqtt-stop zenoh-broker zenoh-stop smoke-mqtt smoke-zenoh clean

help:
	@sed -n '1,16p' Makefile

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

test:
	cargo test

clippy:
	cargo clippy -- -D warnings

check: fmt-check test clippy

run:
	RUST_LOG=$(RUST_LOG) cargo run -- --transport $(TRANSPORT) $(COMMON_ARGS)

run-mqtt:
	RUST_LOG=$(RUST_LOG) cargo run -- --transport mqtt \
		--host $(MQTT_HOST) \
		--port $(MQTT_PORT) \
		--client-id $(MQTT_CLIENT_ID) \
		--qos $(MQTT_QOS) \
		$(COMMON_ARGS)

run-zenoh:
	RUST_LOG=$(RUST_LOG) cargo run -- --transport zenoh \
		--zenoh-endpoint "$(ZENOH_ENDPOINT)" \
		$(COMMON_ARGS)

mqtt-broker:
	@existing="$$(docker ps -aq -f name=^/$(MQTT_CONTAINER)$$)"; \
	if [ -n "$$existing" ]; then \
		mapped="$$(docker port $(MQTT_CONTAINER) 1883/tcp 2>/dev/null | head -1 | sed 's/.*://')"; \
		if [ "$$mapped" != "$(MQTT_PORT)" ]; then \
			echo "recreating $(MQTT_CONTAINER): mapped :$$mapped != wanted :$(MQTT_PORT)"; \
			docker rm -f $(MQTT_CONTAINER) >/dev/null; existing=""; \
		fi; \
	fi; \
	if [ -z "$$existing" ]; then \
		docker run -d --name $(MQTT_CONTAINER) \
			-p $(MQTT_PORT):1883 \
			$(MQTT_IMAGE) mosquitto -c /mosquitto-no-auth.conf >/dev/null; \
		echo "started $(MQTT_CONTAINER): mqtt://127.0.0.1:$(MQTT_PORT)"; \
	else \
		docker start $(MQTT_CONTAINER) >/dev/null; \
		echo "reused $(MQTT_CONTAINER): mqtt://127.0.0.1:$(MQTT_PORT)"; \
	fi

mqtt-stop:
	-@docker rm -f $(MQTT_CONTAINER) >/dev/null 2>&1 || true
	@echo "removed $(MQTT_CONTAINER)"

zenoh-broker:
	$(MAKE) -C $(NEXUS_DIR) zenoh-install

zenoh-stop:
	$(MAKE) -C $(NEXUS_DIR) zenoh-stop

smoke-mqtt:
	@set -e; \
	trap '$(MAKE) mqtt-stop' EXIT; \
	$(MAKE) mqtt-broker; \
	$(MAKE) run-mqtt COUNT=3 INTERVAL_MS=100 MQTT_CLIENT_ID=nexus-datapump-mqtt-smoke

smoke-zenoh:
	@set -e; \
	trap '$(MAKE) zenoh-stop' EXIT; \
	$(MAKE) zenoh-broker; \
	$(MAKE) run-zenoh COUNT=3 INTERVAL_MS=100

clean:
	cargo clean
