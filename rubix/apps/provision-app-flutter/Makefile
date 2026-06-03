# Rubix Provision — Flutter web Makefile
#
# Common targets:
#   make build        Build the web release bundle (build/web)
#   make run          Run the app in Chrome (debug, hot reload)
#   make serve        Serve a built release bundle locally
#   make android      Run on a connected Android phone (debug, hot reload)
#   make apk          Build a release APK
#   make devices      List connected devices
#
# Override the dev port:    make run PORT=8090
# Target a specific phone:  make android ANDROID_DEVICE=<id from `make devices`>

FLUTTER ?= flutter
PORT    ?= 8742
DEVICE  ?= chrome

# Android SDK + adb (works whether or not platform-tools is on PATH).
ANDROID_HOME ?= $(HOME)/Android/Sdk
ADB          ?= $(ANDROID_HOME)/platform-tools/adb
# Auto-pick the first authorized Android device id (flutter's `-d` matches by
# id/name, not by platform). Override for a specific phone:
#   make android ANDROID_DEVICE=R5GYB2T2GYH
ANDROID_DEVICE ?= $(shell $(ADB) devices 2>/dev/null | awk '$$2=="device"{print $$1; exit}')

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

.PHONY: deps
deps: ## Fetch pub dependencies
	$(FLUTTER) pub get

.PHONY: gen
gen: deps ## Run code generation (freezed / riverpod / json_serializable)
	$(FLUTTER) pub run build_runner build --delete-conflicting-outputs

.PHONY: watch
watch: deps ## Run code generation in watch mode
	$(FLUTTER) pub run build_runner watch --delete-conflicting-outputs

.PHONY: run
run: gen ## Run the app in the browser with hot reload (debug)
	$(FLUTTER) run -d $(DEVICE) --web-port $(PORT)

.PHONY: build
build: gen ## Build the web release bundle into build/web
	$(FLUTTER) build web --release

.PHONY: devices
devices: ## List connected Flutter devices (web, desktop, phones)
	$(FLUTTER) devices

.PHONY: android
android: gen ## Run on a connected Android device/emulator with hot reload (debug)
	@test -n "$(ANDROID_DEVICE)" || { echo "No authorized Android device found — plug in, enable USB debugging, approve the prompt, then 'make devices'."; exit 1; }
	$(FLUTTER) run -d $(ANDROID_DEVICE)

.PHONY: apk
apk: gen ## Build a release APK into build/app/outputs/flutter-apk/app-release.apk
	$(FLUTTER) build apk --release

.PHONY: install
install: ## Build + install the release APK onto the connected device
	@test -n "$(ANDROID_DEVICE)" || { echo "No authorized Android device found — plug in, enable USB debugging, approve the prompt, then 'make devices'."; exit 1; }
	$(FLUTTER) install -d $(ANDROID_DEVICE)

.PHONY: serve
serve: build ## Build then serve the release bundle on $(PORT)
	@echo "Serving build/web at http://localhost:$(PORT)"
	@cd build/web && python3 -m http.server $(PORT)

.PHONY: analyze
analyze: ## Run the analyzer
	$(FLUTTER) analyze

.PHONY: clean
clean: ## Remove build artifacts
	$(FLUTTER) clean
