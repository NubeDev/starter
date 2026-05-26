.PHONY: build start kill clean api-client

build:
	flutter build web

start: kill build
	@echo "Starting Flutter web on http://localhost:3031"
	flutter run -d chrome --web-port 3031

kill:
	-pkill -f "flutter run" 2>/dev/null || true
	-pkill -f "flutter_tools" 2>/dev/null || true
	-pkill -f "chrome.*flutter" 2>/dev/null || true

clean:
	flutter clean

# Regenerate the Dart/Dio OpenAPI client at `packages/rubix_api/`
# from `../openapi.json` (kept in sync by the backend). Mirrors the
# TypeScript pattern in `rubix/packages/rubix-client-ts`.
#
# Requires Java (openapi-generator-cli ships as a JAR, downloaded
# on first run by `openapi_generator_cli`).
api-client:
	@echo "==> Regenerating rubix_api Dart/Dio client from ../openapi.json"
	rm -rf packages/rubix_api
	dart run openapi_generator_cli:main generate \
		-i ../openapi.json \
		-g dart-dio \
		-o packages/rubix_api \
		--additional-properties=pubName=rubix_api,pubAuthor=Rubix,pubDescription="Generated Dio client for rubix-agent (DO NOT EDIT — regen via make api-client).",nullableFields=true,useEnumExtension=true
	@echo "==> Running built_value codegen inside packages/rubix_api"
	cd packages/rubix_api && dart pub get && dart run build_runner build
	@echo "==> Refreshing root pub cache"
	flutter pub get
	@echo "==> Done. Generated client is at packages/rubix_api/"
