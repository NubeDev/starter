# Rubix Provision — Flutter web Makefile
#
# Common targets:
#   make build        Build the web release bundle (build/web)
#   make run          Run the app in Chrome (debug, hot reload)
#   make serve        Serve a built release bundle locally
#
# Override the dev port:  make run PORT=8090

FLUTTER ?= flutter
PORT    ?= 8742
DEVICE  ?= chrome

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
