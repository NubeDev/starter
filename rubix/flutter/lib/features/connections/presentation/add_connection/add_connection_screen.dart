import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/network/interfaces/interface_provider.dart';
import 'package:rubix_flutter/core/network/interfaces/interface_selector.dart';
import 'package:rubix_flutter/core/network/lan_scan_controller.dart';
import 'package:rubix_flutter/core/network/lan_scanner.dart';
import 'package:rubix_flutter/features/connections/presentation/add_connection/add_connection_controller.dart';

/// Dev-only placeholder credentials prefilled from a LAN scan hit.
/// Will be removed once real auth-onboarding is wired in.
const _devEmail = 'op@example.com';
const _devPassword = 'rubix-dev-passwd';

class AddConnectionScreen extends ConsumerStatefulWidget {
  const AddConnectionScreen({super.key});

  @override
  ConsumerState<AddConnectionScreen> createState() =>
      _AddConnectionScreenState();
}

class _AddConnectionScreenState extends ConsumerState<AddConnectionScreen> {
  final _formKey = GlobalKey<FormState>();
  final _labelController = TextEditingController();
  final _urlController = TextEditingController();
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _obscurePassword = true;

  @override
  void dispose() {
    _labelController.dispose();
    _urlController.dispose();
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    final success = await ref
        .read(addConnectionControllerProvider.notifier)
        .submit(
          label: _labelController.text.trim(),
          baseUrl: _urlController.text.trim(),
          email: _emailController.text.trim(),
          password: _passwordController.text,
        );
    if (success && mounted) {
      Navigator.of(context).pop();
    }
  }

  void _applyHit(LanHit hit) {
    setState(() {
      _urlController.text = hit.baseUrl;
      if (_labelController.text.trim().isEmpty) {
        _labelController.text = 'rubix @ ${hit.ip}';
      }
      _emailController.text = _devEmail;
      _passwordController.text = _devPassword;
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(addConnectionControllerProvider);

    return Scaffold(
      appBar: AppBar(title: Text(AppLocalizations.of(context).addConnection)),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _formKey,
          child: ListView(
            children: [
              _ScannerPanel(onApply: _applyHit),
              const SizedBox(height: 24),
              TextFormField(
                controller: _urlController,
                decoration: const InputDecoration(
                  labelText: 'URL',
                  hintText: 'http://192.168.1.10:8088',
                  border: OutlineInputBorder(),
                ),
                keyboardType: TextInputType.url,
                autocorrect: false,
                validator: (v) =>
                    (v == null || v.trim().isEmpty) ? 'Required' : null,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _labelController,
                decoration: const InputDecoration(
                  labelText: 'Label',
                  hintText: 'My Agent',
                  border: OutlineInputBorder(),
                ),
                validator: (v) =>
                    (v == null || v.trim().isEmpty) ? 'Required' : null,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _emailController,
                decoration: const InputDecoration(
                  labelText: 'Email',
                  hintText: 'op@example.com',
                  border: OutlineInputBorder(),
                  prefixIcon: Icon(Icons.email_outlined),
                ),
                keyboardType: TextInputType.emailAddress,
                autocorrect: false,
                validator: (v) =>
                    (v == null || v.trim().isEmpty) ? 'Required' : null,
              ),
              const SizedBox(height: 16),
              TextFormField(
                controller: _passwordController,
                obscureText: _obscurePassword,
                validator: (v) =>
                    (v == null || v.isEmpty) ? 'Required' : null,
                decoration: InputDecoration(
                  labelText: 'Password',
                  border: const OutlineInputBorder(),
                  prefixIcon: const Icon(Icons.lock_outline),
                  suffixIcon: IconButton(
                    icon: Icon(
                      _obscurePassword
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined,
                    ),
                    onPressed: () => setState(
                      () => _obscurePassword = !_obscurePassword,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 24),
              if (state is AsyncError)
                Padding(
                  padding: const EdgeInsets.only(bottom: 16),
                  child: Text(
                    state.error.toString(),
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
              FilledButton(
                onPressed: state is AsyncLoading ? null : _submit,
                child: state is AsyncLoading
                    ? const SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : Text(AppLocalizations.of(context).probeAndSave),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Optional LAN scanner: pick an interface, hit Scan, tap a hit to
/// prefill the form below.
class _ScannerPanel extends ConsumerStatefulWidget {
  const _ScannerPanel({required this.onApply});

  final ValueChanged<LanHit> onApply;

  @override
  ConsumerState<_ScannerPanel> createState() => _ScannerPanelState();
}

class _ScannerPanelState extends ConsumerState<_ScannerPanel> {
  bool _expanded = false;
  final _portController = TextEditingController(text: '8088');

  @override
  void dispose() {
    _portController.dispose();
    super.dispose();
  }

  void _startScan() {
    final iface = ref.read(selectedNetworkInterfaceProvider);
    final ip = iface?.primaryAddress;
    if (ip == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Select an interface with an IPv4 address first'),
        ),
      );
      return;
    }
    final port = int.tryParse(_portController.text.trim()) ?? 8088;
    ref.read(lanScanControllerProvider.notifier).start(
          localIp: ip,
          port: port,
        );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scanState = ref.watch(lanScanControllerProvider);

    return Card(
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: theme.colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            InkWell(
              onTap: () => setState(() => _expanded = !_expanded),
              child: Row(
                children: [
                  Icon(
                    _expanded ? Icons.expand_less : Icons.expand_more,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'Scan local network (optional)',
                      style: theme.textTheme.titleSmall,
                    ),
                  ),
                  if (scanState is LanScanRunning)
                    const SizedBox(
                      height: 16,
                      width: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                ],
              ),
            ),
            if (_expanded) ...[
              const SizedBox(height: 12),
              if (kIsWeb)
                const _WebUnsupportedNotice()
              else ...[
                const NetworkInterfaceSelector(),
                const SizedBox(height: 12),
                Row(
                  children: [
                    SizedBox(
                      width: 100,
                      child: TextFormField(
                        controller: _portController,
                        keyboardType: TextInputType.number,
                        decoration: const InputDecoration(
                          labelText: 'Port',
                          border: OutlineInputBorder(),
                          isDense: true,
                        ),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: FilledButton.tonalIcon(
                        onPressed: scanState is LanScanRunning
                            ? () => ref
                                .read(lanScanControllerProvider.notifier)
                                .cancel()
                            : _startScan,
                        icon: Icon(
                          scanState is LanScanRunning
                              ? Icons.stop
                              : Icons.radar,
                        ),
                        label: Text(
                          scanState is LanScanRunning ? 'Stop' : 'Scan',
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                _ScanResults(state: scanState, onApply: widget.onApply),
              ],
            ],
          ],
        ),
      ),
    );
  }
}

class _ScanResults extends StatelessWidget {
  const _ScanResults({required this.state, required this.onApply});

  final LanScanState state;
  final ValueChanged<LanHit> onApply;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    switch (state) {
      case LanScanIdle():
        return Text(
          'Pick an interface and press Scan to discover rubix-agent '
          'instances on this /24 subnet.',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.outline,
          ),
        );
      case LanScanFailed(:final message):
        return Text(
          message,
          style: TextStyle(color: theme.colorScheme.error),
        );
      case LanScanRunning(:final progress):
        return _ProgressAndHits(
          progress: progress,
          onApply: onApply,
          running: true,
        );
      case LanScanDone(:final progress):
        return _ProgressAndHits(
          progress: progress,
          onApply: onApply,
          running: false,
        );
    }
  }
}

class _ProgressAndHits extends StatelessWidget {
  const _ProgressAndHits({
    required this.progress,
    required this.onApply,
    required this.running,
  });

  final LanScanProgress progress;
  final ValueChanged<LanHit> onApply;
  final bool running;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final hits = progress.hits;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        LinearProgressIndicator(
          value: progress.total == 0 ? null : progress.percent,
        ),
        const SizedBox(height: 6),
        Text(
          running
              ? '${progress.scanned}/${progress.total}'
                  '${progress.lastIp != null ? '  ·  ${progress.lastIp}' : ''}'
              : 'Done — ${hits.length} found',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.outline,
          ),
        ),
        const SizedBox(height: 8),
        if (hits.isEmpty && !running)
          Text(
            'No rubix-agent instances responded.',
            style: theme.textTheme.bodySmall,
          )
        else
          ...hits.map(
            (h) => Card(
              margin: const EdgeInsets.only(bottom: 6),
              child: ListTile(
                dense: true,
                leading: const Icon(Icons.dns_outlined),
                title: Text(h.baseUrl),
                subtitle: h.version != null ? Text('v${h.version}') : null,
                trailing: const Icon(Icons.arrow_forward_ios, size: 14),
                onTap: () => onApply(h),
              ),
            ),
          ),
      ],
    );
  }
}

/// Shown in place of the scanner UI on Flutter web, where
/// `dart:io`'s `NetworkInterface.list` / `Socket` aren't available
/// and the browser would block LAN probes via CORS / mixed-content
/// anyway. Operators should enter the URL by hand or run the
/// scanner from the desktop / mobile build.
class _WebUnsupportedNotice extends StatelessWidget {
  const _WebUnsupportedNotice();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.info_outline, color: theme.colorScheme.outline),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              'LAN scan is not available in the browser. Run rubix '
              'from the desktop or mobile app to discover agents '
              'automatically, or enter the URL by hand below.',
              style: theme.textTheme.bodySmall,
            ),
          ),
        ],
      ),
    );
  }
}
