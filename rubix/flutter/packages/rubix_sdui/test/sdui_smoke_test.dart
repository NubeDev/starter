// Smoke test — verifies the package's public surface imports and
// the sealed `SduiComponent` dispatcher degrades unknown types to
// `DanglingComponent` (R7 capability filter mirror).

import 'package:flutter_test/flutter_test.dart';
import 'package:rubix_sdui/rubix_sdui.dart';

void main() {
  test('kSupportedIrVersion is pinned to 5', () {
    expect(kSupportedIrVersion, 5);
  });

  test('SduiComponent.fromJson parses a known variant', () {
    final c = SduiComponent.fromJson({'type': 'text', 'id': 'hello'});
    expect(c, isA<TextComponent>());
    expect(c.id, 'hello');
  });

  test('SduiComponent.fromJson degrades unknown variants to Dangling', () {
    final c = SduiComponent.fromJson({'type': 'never_seen', 'id': 'x'});
    expect(c, isA<DanglingComponent>());
    expect((c as DanglingComponent).reason, 'unknown:never_seen');
  });

  test('BindingSpec parses both short and full forms', () {
    final s = BindingSpec.fromJson(r'$target.enabled');
    expect(s, isA<ShortBinding>());

    final f = BindingSpec.fromJson({
      'slot': r'$target.enabled',
      'concurrency': 'occ',
      'debounce_ms': 50,
    });
    expect(f, isA<FullBinding>());
    expect((f as FullBinding).concurrency, SduiConcurrency.occ);
  });

  test('ComponentTree round-trips ir_version + root.type', () {
    final tree = ComponentTree.fromJson({
      'ir_version': 5,
      'root': {'type': 'page', 'id': 'root'},
    });
    expect(tree.irVersion, 5);
    expect(tree.root, isA<PageComponent>());
    expect(tree.toJson()['ir_version'], 5);
  });
}
