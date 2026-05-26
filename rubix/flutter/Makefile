.PHONY: build start kill clean

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
