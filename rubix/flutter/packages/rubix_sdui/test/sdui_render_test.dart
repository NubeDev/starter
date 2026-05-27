// Render test — feeds a hand-built tree (page > row > col[span:6]
// > kpi, plus a chart row) through `SduiRenderer` under `pumpWidget`
// and asserts the title, KPI label, KPI value, and chart title
// appear. No HTTP — the service stays at the notifier seam.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:rubix_sdui/rubix_sdui.dart';

class _FakeService implements SduiService {
  _FakeService(this._result);
  final SduiResolveResult _result;

  @override
  Future<SduiResolveResult> resolve(ResolveRequest request) async => _result;

  @override
  Future<SduiActionResponse> dispatchAction(
    SduiAction action, {
    Map<String, Object?> context = const {},
  }) async =>
      throw UnimplementedError();

  @override
  Future<TableResponse> queryTable(TableQuery query) async =>
      throw UnimplementedError();
}

void main() {
  testWidgets(
    'renders page title + KPI + chart from a fixture tree',
    (tester) async {
      final tree = ComponentTree.fromJson({
        'ir_version': 5,
        'root': {
          'type': 'page',
          'id': 'root',
          'title': 'Site A — energy + water',
          'children': [
            {
              'type': 'row',
              'id': 'kpis',
              'children': [
                {
                  'type': 'col',
                  'span': 6,
                  'children': [
                    {
                      'type': 'kpi',
                      'id': 'kpi-kwh',
                      'label': 'Site A — last 24h kWh',
                      'value': 10002.42,
                      'format': 'number',
                      'unit_symbol': 'kWh',
                    },
                  ],
                },
              ],
            },
            {
              'type': 'row',
              'id': 'charts',
              'children': [
                {
                  'type': 'col',
                  'children': [
                    {
                      'type': 'chart',
                      'id': 'chart-elec',
                      'title': 'Electricity — main (30d, 15m)',
                      'kind': 'line',
                      'series': [
                        {
                          'label': 'meter',
                          'points': [
                            [1779801300000, 10005.22],
                            [1779802200000, 10003.66],
                            [1779803100000, 10004.84],
                          ],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      });

      final notifier = SduiNotifier(
        service: _FakeService(
          SduiResolveResult(tree: tree, subscriptions: const []),
        ),
      );
      await notifier.load(pageRef: 'fixture');

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SduiProvider(
              notifier: notifier,
              child: const SduiRenderer(),
            ),
          ),
        ),
      );
      await tester.pump();

      expect(find.text('Site A — energy + water'), findsOneWidget);
      expect(find.text('Site A — last 24h kWh'), findsOneWidget);
      expect(find.text('10002.42'), findsOneWidget);
      expect(find.text('kWh'), findsOneWidget);
      expect(find.text('Electricity — main (30d, 15m)'), findsOneWidget);

      notifier.dispose();
    },
  );
}
