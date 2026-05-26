import 'package:flutter_test/flutter_test.dart';
import 'package:rubix_flutter/features/home/domain/me_response/me_response.dart';

void main() {
  group('MeResponse', () {
    test('parses backend wire shape', () {
      final me = MeResponse.fromJson({
        'subject': 'user_abc',
        'email': 'op@example.com',
        'role': 'admin',
      });
      expect(me.subject, 'user_abc');
      expect(me.email, 'op@example.com');
      expect(me.role, 'admin');
    });

    test('round-trips through toJson', () {
      const me = MeResponse(
        subject: 'u1',
        email: 'a@b.c',
        role: 'reader',
      );
      expect(MeResponse.fromJson(me.toJson()), me);
    });
  });
}
