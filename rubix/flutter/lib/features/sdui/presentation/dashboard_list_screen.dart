import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';
import 'package:rubix_flutter/core/network/sse_client.dart';

/// Lists dashboards for the `system` tenant by consuming the
/// `GET /api/v1/dashboards/events` SSE stream. The first frame
/// is a `snapshot` that seeds the list; subsequent `created` /
/// `updated` / `deleted` frames mutate it live.
class DashboardListScreen extends ConsumerStatefulWidget {
  const DashboardListScreen({super.key});

  @override
  ConsumerState<DashboardListScreen> createState() =>
      _DashboardListScreenState();
}

class _DashboardListScreenState extends ConsumerState<DashboardListScreen> {
  StreamSubscription<String>? _sub;
  CancelToken? _cancel;
  final Map<String, _DashboardItem> _items = {};
  Object? _error;
  bool _connecting = true;
  bool _hasSnapshot = false;

  @override
  void initState() {
    super.initState();
    final dio = ref.read(dioProvider);
    if (dio != null) _open(dio);
  }

  void _open(Dio dio) {
    _cancel?.cancel();
    _sub?.cancel();
    setState(() {
      _connecting = true;
      _error = null;
      _hasSnapshot = false;
    });

    final cancel = CancelToken();
    _cancel = cancel;
    final stream = SseClient(dio: dio).connect(
      '/api/v1/dashboards/events',
      cancelToken: cancel,
    );

    _sub = stream.listen(
      _handleFrame,
      onError: (Object e) {
        if (!mounted) return;
        setState(() {
          _error = e;
          _connecting = false;
        });
      },
      onDone: () {
        if (!mounted) return;
        setState(() => _connecting = false);
      },
    );
  }

  void _handleFrame(String raw) {
    if (raw.isEmpty) return;
    final Object? decoded;
    try {
      decoded = jsonDecode(raw);
    } catch (_) {
      return;
    }
    if (decoded is! Map) return;
    final map = decoded.cast<String, Object?>();
    final kind = map['kind'] as String?;
    if (kind == null) return;

    setState(() {
      _connecting = false;
      _error = null;
      switch (kind) {
        case 'snapshot':
          _items.clear();
          final items = map['items'];
          if (items is List) {
            for (final item in items.whereType<Map<Object?, Object?>>()) {
              final parsed =
                  _DashboardItem.fromJson(item.cast<String, Object?>());
              _items[parsed.pageId] = parsed;
            }
          }
          _hasSnapshot = true;
        case 'created':
        case 'updated':
          final parsed = _DashboardItem.fromJson(map);
          if (parsed.pageId.isEmpty) return;
          final existing = _items[parsed.pageId];
          _items[parsed.pageId] = _DashboardItem(
            pageId: parsed.pageId,
            // `updated` frames may omit `title` if it didn't
            // change — preserve the prior title in that case.
            title: parsed.title.isEmpty && existing != null
                ? existing.title
                : parsed.title,
          );
        case 'deleted':
          final pageId = map['page_id'] as String?;
          if (pageId != null) _items.remove(pageId);
      }
    });
  }

  @override
  void dispose() {
    _sub?.cancel();
    _cancel?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final dio = ref.watch(dioProvider);
    final theme = Theme.of(context);

    if (dio == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Dashboards')),
        body: const Center(
          child: Padding(
            padding: EdgeInsets.all(24),
            child: Text(
              'No active connection — add one in Connections first.',
              textAlign: TextAlign.center,
            ),
          ),
        ),
      );
    }

    final sorted = _items.values.toList()
      ..sort((a, b) => a.title.toLowerCase().compareTo(b.title.toLowerCase()));

    return Scaffold(
      appBar: AppBar(
        title: const Text('Dashboards'),
        actions: [
          IconButton(
            tooltip: 'Reconnect',
            icon: const Icon(Icons.refresh),
            onPressed: () => _open(dio),
          ),
        ],
      ),
      body: Builder(
        builder: (context) {
          if (!_hasSnapshot && _error == null && _connecting) {
            return const Center(child: CircularProgressIndicator());
          }
          if (_error != null && !_hasSnapshot) {
            return _ErrorView(error: _error, onRetry: () => _open(dio));
          }
          if (sorted.isEmpty) {
            return const Center(child: Text('No dashboards yet'));
          }
          return ListView.separated(
            padding: const EdgeInsets.symmetric(vertical: 8),
            itemCount: sorted.length,
            separatorBuilder: (_, __) => const Divider(height: 1),
            itemBuilder: (context, i) {
              final item = sorted[i];
              return ListTile(
                title: Text(item.title),
                subtitle: Text(
                  item.pageId,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                trailing: const Icon(Icons.chevron_right),
                onTap: () => context.push('/sdui/${item.pageId}'),
              );
            },
          );
        },
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.error, required this.onRetry});
  final Object? error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Failed to load dashboards:\n${error ?? "unknown error"}',
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 16),
            FilledButton(onPressed: onRetry, child: const Text('Retry')),
          ],
        ),
      ),
    );
  }
}

class _DashboardItem {
  const _DashboardItem({required this.pageId, required this.title});

  factory _DashboardItem.fromJson(Map<String, Object?> map) => _DashboardItem(
        pageId: map['page_id'] as String? ?? '',
        title: map['title'] as String? ?? '',
      );

  final String pageId;
  final String title;
}
