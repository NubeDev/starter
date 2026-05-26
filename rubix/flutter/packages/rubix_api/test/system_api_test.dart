import 'package:test/test.dart';
import 'package:rubix_api/rubix_api.dart';


/// tests for SystemApi
void main() {
  final instance = RubixApi().getSystemApi();

  group(SystemApi, () {
    // Handler — kept at ≤20 lines. Any growth here is a smell: domain logic belongs in `rubix-tools` (push into `probe()`), shaping logic belongs in [`shape_response`].
    //
    //Future dispatch(String toolId, JsonObject body, { String render }) async
    test('test dispatch', () async {
      // TODO
    });

    // Liveness probe. Returns 200 with a tiny JSON body — no DB, no downstream calls. A reachable port is the entire signal.
    //
    //Future healthz() async
    test('test healthz', () async {
      // TODO
    });

  });
}
