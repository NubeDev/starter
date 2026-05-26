import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';
import 'package:rubix_flutter/features/connections/presentation/edit_connection/edit_connection_controller.dart';

class EditConnectionScreen extends ConsumerStatefulWidget {
  const EditConnectionScreen({required this.connection, super.key});

  final Connection connection;

  @override
  ConsumerState<EditConnectionScreen> createState() =>
      _EditConnectionScreenState();
}

class _EditConnectionScreenState extends ConsumerState<EditConnectionScreen> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _labelController;

  @override
  void initState() {
    super.initState();
    _labelController = TextEditingController(text: widget.connection.label);
  }

  @override
  void dispose() {
    _labelController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    final success = await ref
        .read(editConnectionControllerProvider.notifier)
        .update(widget.connection.id, label: _labelController.text.trim());
    if (success && mounted) Navigator.of(context).pop();
  }

  Future<void> _confirmDelete() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(AppLocalizations.of(context).deleteConnection),
        content: Text(AppLocalizations.of(context).deleteConnectionConfirm),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: Text(AppLocalizations.of(context).cancel),
          ),
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(true),
            child: Text(AppLocalizations.of(context).delete),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await ref
          .read(editConnectionControllerProvider.notifier)
          .delete(widget.connection.id);
      if (mounted) Navigator.of(context).pop();
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(editConnectionControllerProvider);

    return Scaffold(
      appBar: AppBar(
        title: Text(AppLocalizations.of(context).editConnection),
        actions: [
          IconButton(
            icon: const Icon(Icons.delete),
            onPressed: _confirmDelete,
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              TextFormField(
                controller: _labelController,
                decoration: const InputDecoration(labelText: 'Label'),
                validator: (v) =>
                    (v == null || v.trim().isEmpty) ? 'Required' : null,
              ),
              const SizedBox(height: 16),
              Text(
                widget.connection.baseUrl,
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const SizedBox(height: 24),
              FilledButton(
                onPressed: state is AsyncLoading ? null : _save,
                child: Text(AppLocalizations.of(context).save),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
