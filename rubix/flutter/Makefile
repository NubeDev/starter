.PHONY: build start kill clean api-client server start-web

build:
	flutter build web

start: kill build
	@echo "Starting Flutter web on http://localhost:3031"
	flutter run -d chrome --web-port 3031

# Run the Dart REST backend that the Flutter web build talks to.
# On native (mobile/desktop) the app uses Drift directly and does not
# need this server. Default port: 8787.
server:
	cd packages/rubix_server && dart run bin/server.dart

# Convenience: launch the server in the background, then start Flutter
# web pointed at it with the matching bearer token from ~/.rubix/server.token.
# Server logs land in /tmp/rubix_server.log.
start-web: kill
	@echo "==> Starting rubix_server on :8787 (logs: /tmp/rubix_server.log)"
	@cd packages/rubix_server && dart run bin/server.dart > /tmp/rubix_server.log 2>&1 &
	@# Wait until the token file is written by the server's first boot.
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
	  if [ -s "$$HOME/.rubix/server.token" ]; then break; fi; \
	  sleep 0.3; \
	done
	@if [ ! -s "$$HOME/.rubix/server.token" ]; then \
	  echo "rubix_server did not write a token; check /tmp/rubix_server.log"; \
	  exit 1; \
	fi
	@echo "==> Starting Flutter web on :3031"
	flutter run -d chrome --web-port 3031 \
	  --dart-define=RUBIX_SERVER_URL=http://localhost:8787 \
	  --dart-define=RUBIX_SERVER_TOKEN=$$(cat $$HOME/.rubix/server.token)

kill:
	-pkill -f "flutter run" 2>/dev/null || true
	-pkill -f "flutter_tools" 2>/dev/null || true
	-pkill -f "chrome.*flutter" 2>/dev/null || true
	-pkill -f "rubix_server/bin/server.dart" 2>/dev/null || true

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
	@# Seed a placeholder pubspec so `dart pub get` inside this
	@# workspace (triggered by `dart run openapi_generator_cli:main`)
	@# can resolve the `rubix_api: path:` dep that the root
	@# pubspec.yaml declares. The generator overwrites the file in
	@# the next step.
	mkdir -p packages/rubix_api
	@printf 'name: rubix_api\nversion: 0.0.0\nenvironment:\n  sdk: ">=2.15.0 <4.0.0"\n' \
		> packages/rubix_api/pubspec.yaml
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
