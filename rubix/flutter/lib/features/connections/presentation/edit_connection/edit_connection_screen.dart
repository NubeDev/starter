import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:rubix_flutter/features/connections/domain/connection/connection.dart';
import 'package:rubix_flutter/features/connections/presentation/edit_connection/edit_connection_controller.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

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
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _obscurePassword = true;
  bool _credsLoaded = false;

  @override
  void initState() {
    super.initState();
    _labelController = TextEditingController(text: widget.connection.label);
    _loadCreds();
  }

  Future<void> _loadCreds() async {
    final creds = await ref
        .read(connectionCredentialsStoreProvider)
        .read(widget.connection.id);
    if (!mounted) return;
    setState(() {
      if (creds != null) {
        _emailController.text = creds.email;
        _passwordController.text = creds.password;
      }
      _credsLoaded = true;
    });
  }

  @override
  void dispose() {
    _labelController.dispose();
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    final email = _emailController.text.trim();
    final password = _passwordController.text;
    final success = await ref
        .read(editConnectionControllerProvider.notifier)
        .update(
          widget.connection.id,
          label: _labelController.text.trim(),
          email: email.isEmpty ? null : email,
          password: password.isEmpty ? null : password,
        );
    if (success && mounted) Navigator.of(context).pop();
  }

  Future<void> _confirmDelete() async {
    final l = AppLocalizations.of(context);
    final confirmed = await showNubeConfirmDialog(
      context,
      title: l.deleteConnection,
      message: l.deleteConnectionConfirm,
      confirmLabel: l.delete,
      cancelLabel: l.cancel,
      destructive: true,
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
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final state = ref.watch(editConnectionControllerProvider);

    return Scaffold(
      backgroundColor: t.bg,
      body: SafeArea(
        child: Column(
          children: [
            NubeTopBar(
              title: l.editConnection,
              actions: [
                NubeTopBarAction(
                  icon: LucideIcons.trash2,
                  tooltip: l.delete,
                  danger: true,
                  onTap: _confirmDelete,
                ),
              ],
            ),
            Expanded(
              child: !_credsLoaded
                  ? const LoadingIndicator()
                  : Form(
                      key: _formKey,
                      child: ListView(
                        padding: const EdgeInsets.all(20),
                        children: [
                          NubeField(
                            controller: _labelController,
                            label: 'Label',
                            validator: (v) => (v == null || v.trim().isEmpty)
                                ? 'Required'
                                : null,
                          ),
                          const SizedBox(height: 12),
                          Text(
                            widget.connection.baseUrl,
                            style: TextStyle(
                              color: t.muted,
                              fontSize: 12.5,
                              fontFamily: 'monospace',
                            ),
                          ),
                          const SizedBox(height: 20),
                          NubeField(
                            controller: _emailController,
                            label: 'Email',
                            prefixIcon: LucideIcons.mail,
                            keyboardType: TextInputType.emailAddress,
                            autocorrect: false,
                          ),
                          const SizedBox(height: 16),
                          NubeField(
                            controller: _passwordController,
                            label: 'Password',
                            obscureText: _obscurePassword,
                            prefixIcon: LucideIcons.lock,
                            suffixIcon: _obscurePassword
                                ? LucideIcons.eye
                                : LucideIcons.eyeOff,
                            onSuffixTap: () => setState(
                              () => _obscurePassword = !_obscurePassword,
                            ),
                          ),
                          const SizedBox(height: 20),
                          NubeButton(
                            label: l.save,
                            icon: LucideIcons.check,
                            expand: true,
                            loading: state is AsyncLoading,
                            onPressed: state is AsyncLoading ? null : _save,
                          ),
                        ],
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}
