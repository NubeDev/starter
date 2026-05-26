import 'package:test/test.dart';
import 'package:rubix_api/rubix_api.dart';


/// tests for AuthApi
void main() {
  final instance = RubixApi().getAuthApi();

  group(AuthApi, () {
    //Future<TokenResponse> issueToken(TokenRequest tokenRequest) async
    test('test issueToken', () async {
      // TODO
    });

    //Future<LoginResponse> login(LoginRequest loginRequest) async
    test('test login', () async {
      // TODO
    });

    //Future logout() async
    test('test logout', () async {
      // TODO
    });

    //Future<MeResponse> me() async
    test('test me', () async {
      // TODO
    });

  });
}
