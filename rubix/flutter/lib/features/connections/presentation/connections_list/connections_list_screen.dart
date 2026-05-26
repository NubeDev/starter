import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';
import 'package:rubix_flutter/features/connections/presentation/add_connection/add_connection_screen.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/connections/presentation/edit_connection/edit_connection_screen.dart';

class ConnectionsListScreen extends ConsumerWidget {
  const ConnectionsListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final listAsync = ref.watch(connectionListControllerProvider);

    return Scaffold(
      floatingActionButton: FloatingActionButton(
        onPressed: () => Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (_) => const AddConnectionScreen(),
          ),
        ),
        child: const Icon(Icons.add),
      ),
      body: listAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Text('${AppLocalizations.of(context).error}: $e'),
        ),
        data: (connections) => connections.isEmpty
            ? Center(child: Text(AppLocalizations.of(context).noConnections))
            : ListView.builder(
                itemCount: connections.length,
                itemBuilder: (context, index) =>
                    _ConnectionTile(connection: connections[index]),
              ),
      ),
    );
  }
}

class _ConnectionTile extends ConsumerWidget {
  const _ConnectionTile({required this.connection});

  final Connection connection;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeAsync = ref.watch(activeConnectionProvider);
    final isActive = activeAsync.value?.id == connection.id;

    return Dismissible(
      key: ValueKey(connection.id),
      direction: DismissDirection.endToStart,
      background: Container(
        alignment: Alignment.centerRight,
        padding: const EdgeInsets.only(right: 16),
        color: Colors.red,
        child: const Icon(Icons.delete, color: Colors.white),
      ),
      onDismissed: (_) => ref
          .read(connectionListControllerProvider.notifier)
          .delete(connection.id),
      child: ListTile(
        leading: Icon(
          isActive ? Icons.check_circle : Icons.circle_outlined,
          color: isActive ? Colors.green : null,
        ),
        title: Text(connection.label),
        subtitle: Text(connection.baseUrl),
        trailing: IconButton(
          icon: const Icon(Icons.edit_outlined),
          tooltip: 'Edit',
          onPressed: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => EditConnectionScreen(connection: connection),
            ),
          ),
        ),
        onTap: () async {
          final messenger = ScaffoldMessenger.of(context);
          try {
            await ref
                .read(connectionListControllerProvider.notifier)
                .activate(connection.id);
          } catch (e) {
            messenger.showSnackBar(
              SnackBar(content: Text('Sign-in failed: $e')),
            );
          }
        },
        onLongPress: () => Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (_) => EditConnectionScreen(connection: connection),
          ),
        ),
      ),
    );
  }
}
