import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';

/// Lists dashboards for the `system` tenant via
/// `POST /api/v1/tools/rubix.dashboard.list`.
class DashboardListScreen extends ConsumerStatefulWidget {
  const DashboardListScreen({super.key});

  @override
  ConsumerState<DashboardListScreen> createState() =>
      _DashboardListScreenState();
}

class _DashboardListScreenState extends ConsumerState<DashboardListScreen> {
  List<_DashboardItem>? _items;
  Object? _error;
  bool _loading = false;
  String? _boundToken;

  Future<void> _load(Dio dio) async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final res = await dio.post<Map<String, dynamic>>(
        '/api/v1/tools/rubix.dashboard.list',
        data: const {'tenant_id': 'system'},
      );
      final data = res.data ?? const <String, dynamic>{};
      final raw = data['items'];
      final parsed = <_DashboardItem>[];
      if (raw is List) {
        for (final item in raw) {
          if (item is Map) {
            parsed.add(
              _DashboardItem.fromJson(item.cast<String, Object?>()),
            );
          }
        }
      }
      parsed.sort(
        (a, b) => a.title.toLowerCase().compareTo(b.title.toLowerCase()),
      );
      if (!mounted) return;
      setState(() {
        _items = parsed;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e;
        _loading = false;
      });
    }
  }

  void _retry(Dio dio) {
    _boundToken = null;
    // Re-run AuthController.build() — that path silently re-issues a
    // token from saved creds if there is one, or transitions to
    // Unauthenticated otherwise.
    ref.invalidate(authControllerProvider);
    setState(() {
      _items = null;
      _error = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final dio = ref.watch(dioProvider);
    final authAsync = ref.watch(authControllerProvider);
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

    final authState = authAsync.value;
    final token = authState is AuthAuthenticated ? authState.token : null;
    if (token != null && token != _boundToken) {
      _boundToken = token;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _load(dio);
      });
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('Dashboards'),
        actions: [
          IconButton(
            tooltip: 'Reload',
            icon: const Icon(Icons.refresh),
            onPressed: () => _load(dio),
          ),
        ],
      ),
      body: Builder(
        builder: (context) {
          if (authAsync.isLoading ||
              (_items == null && _error == null && (token == null || _loading))) {
            return const Center(child: CircularProgressIndicator());
          }
          if (token == null) {
            final reason = authState is AuthUnauthenticated
                ? authState.reason
                : null;
            return _ErrorView(
              error: reason == null
                  ? 'Not signed in to this connection.'
                  : 'Not signed in: $reason',
              onRetry: () => _retry(dio),
            );
          }
          if (_error != null) {
            return _ErrorView(error: _error, onRetry: () => _load(dio));
          }
          final items = _items ?? const <_DashboardItem>[];
          if (items.isEmpty) {
            return const Center(child: Text('No dashboards yet'));
          }
          return ListView.separated(
            padding: const EdgeInsets.symmetric(vertical: 8),
            itemCount: items.length,
            separatorBuilder: (_, __) => const Divider(height: 1),
            itemBuilder: (context, i) {
              final item = items[i];
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
