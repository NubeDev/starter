import 'package:test/test.dart';
import 'package:rubix_api/rubix_api.dart';

// tests for PasswordNotSetResponse
void main() {
  final instance = PasswordNotSetResponseBuilder();
  // TODO add properties to the builder and call build()

  group(PasswordNotSetResponse, () {
    // Always `\"password_not_set\"`. Discriminator field; lets clients pattern-match without inspecting the HTTP status alone.
    // String error
    test('to test the property `error`', () async {
      // TODO
    });

    // Provider ids the user has linked. Empty list when no third-party path is configured (the default [`crate::NoLinkedProviders`] impl).
    // BuiltList<String> providers
    test('to test the property `providers`', () async {
      // TODO
    });

  });
}
