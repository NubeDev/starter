/// Block 5 integration smoke test.
///
/// Exercises the full happy path:
///   1. Cold-start → connections list shown.
///   2. Add a connection via form.
///   3. Redirected to login → sign in with provided creds.
///   4. Home screen renders: green status pill + user email.
///
/// Run with:
/// ```sh
/// flutter test integration_test/smoke_test.dart \
///   --dart-define=RUBIX_URL=http://127.0.0.1:8088 \
///   --dart-define=RUBIX_EMAIL=op@example.com \
///   --dart-define=RUBIX_PASSWORD=rubix-dev-passwd
/// ```
library;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:rubix_flutter/main.dart' as app;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  const url = String.fromEnvironment('RUBIX_URL');
  const email = String.fromEnvironment('RUBIX_EMAIL');
  const password = String.fromEnvironment('RUBIX_PASSWORD');

  assert(url.isNotEmpty, 'RUBIX_URL must be set via --dart-define');
  assert(email.isNotEmpty, 'RUBIX_EMAIL must be set via --dart-define');
  assert(password.isNotEmpty, 'RUBIX_PASSWORD must be set via --dart-define');

  testWidgets('full happy-path: connection → login → home', (tester) async {
    app.main();
    await tester.pumpAndSettle();

    // ─── Step 1: Connections list (cold start, no connections) ──────────
    // We should see the "Add Connection" button or the connections list.
    // Tap the FAB / add button to open Add Connection.
    final addBtn = find.byIcon(Icons.add);
    expect(addBtn, findsOneWidget, reason: 'Connections list FAB missing');
    await tester.tap(addBtn);
    await tester.pumpAndSettle();

    // ─── Step 2: Fill "Add Connection" form ────────────────────────────
    final urlField = find.widgetWithText(TextFormField, 'URL');
    expect(urlField, findsOneWidget);
    await tester.enterText(urlField, url);

    final labelField = find.widgetWithText(TextFormField, 'Label');
    expect(labelField, findsOneWidget);
    await tester.enterText(labelField, 'E2E Agent');

    final saveBtn = find.widgetWithText(FilledButton, 'Probe & Save');
    await tester.tap(saveBtn);
    await tester.pumpAndSettle(const Duration(seconds: 6));

    // After save we return to connections list; activate it.
    final tile = find.text('E2E Agent');
    expect(tile, findsOneWidget, reason: 'Connection not in list');
    await tester.tap(tile);
    await tester.pumpAndSettle();

    // ─── Step 3: Login screen ──────────────────────────────────────────
    // Router redirect should land us on /login.
    final emailField = find.widgetWithText(TextFormField, 'Email');
    expect(emailField, findsOneWidget, reason: 'Login screen not shown');
    await tester.enterText(emailField, email);

    final pwField = find.widgetWithText(TextFormField, 'Password');
    await tester.enterText(pwField, password);

    final signInBtn = find.widgetWithText(FilledButton, 'Sign In');
    await tester.tap(signInBtn);
    await tester.pumpAndSettle(const Duration(seconds: 6));

    // ─── Step 4: Home screen assertions ────────────────────────────────
    // Status pill should show "Agent online" (green).
    expect(find.text('Agent online'), findsOneWidget,
        reason: 'Health pill not showing online');

    // User email rendered.
    expect(find.text(email), findsOneWidget,
        reason: 'User email not on home screen');

    // Active connection label visible.
    expect(find.text('E2E Agent'), findsOneWidget,
        reason: 'Connection label missing from home');
  });
}
