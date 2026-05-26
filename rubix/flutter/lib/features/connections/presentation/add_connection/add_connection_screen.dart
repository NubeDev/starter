import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/network/interfaces/interface_provider.dart';
import 'package:rubix_flutter/core/network/interfaces/interface_selector.dart';
import 'package:rubix_flutter/core/network/lan_scan_controller.dart';
import 'package:rubix_flutter/core/network/lan_scanner.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/connections/presentation/add_connection/add_connection_controller.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// Dev-only placeholder credentials prefilled from a LAN scan hit.
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
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final state = ref.watch(addConnectionControllerProvider);

    return Scaffold(
      backgroundColor: t.bg,
      body: SafeArea(
        child: Column(
          children: [
            NubeTopBar(title: l.addConnection),
            Expanded(
              child: Form(
                key: _formKey,
                child: ListView(
                  padding: const EdgeInsets.all(20),
                  children: [
                    _ScannerPanel(onApply: _applyHit),
                    const SizedBox(height: 20),
                    NubeField(
                      controller: _urlController,
                      label: 'URL',
                      hint: 'http://192.168.1.10:8088',
                      keyboardType: TextInputType.url,
                      autocorrect: false,
                      validator: (v) => (v == null || v.trim().isEmpty)
                          ? 'Required'
                          : null,
                    ),
                    const SizedBox(height: 16),
                    NubeField(
                      controller: _labelController,
                      label: 'Label',
                      hint: 'My Agent',
                      validator: (v) => (v == null || v.trim().isEmpty)
                          ? 'Required'
                          : null,
                    ),
                    const SizedBox(height: 16),
                    NubeField(
                      controller: _emailController,
                      label: 'Email',
                      hint: 'op@example.com',
                      prefixIcon: LucideIcons.mail,
                      keyboardType: TextInputType.emailAddress,
                      autocorrect: false,
                      validator: (v) => (v == null || v.trim().isEmpty)
                          ? 'Required'
                          : null,
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
                      validator: (v) =>
                          (v == null || v.isEmpty) ? 'Required' : null,
                    ),
                    const SizedBox(height: 20),
                    if (state is AsyncError) ...[
                      NubeAlert.error(state.error.toString()),
                      const SizedBox(height: 14),
                    ],
                    NubeButton(
                      label: l.probeAndSave,
                      icon: LucideIcons.plug,
                      expand: true,
                      loading: state is AsyncLoading,
                      onPressed: state is AsyncLoading ? null : _submit,
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
    final t = Theme.of(context).nube;
    final scanState = ref.watch(lanScanControllerProvider);

    return NubeCard(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          MouseRegion(
            cursor: SystemMouseCursors.click,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => setState(() => _expanded = !_expanded),
              child: Row(
                children: [
                  Icon(
                    _expanded ? LucideIcons.chevronDown : LucideIcons.chevronRight,
                    size: 16,
                    color: t.muted,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'Scan local network',
                      style: TextStyle(
                        color: t.text,
                        fontSize: 13.5,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  if (scanState is LanScanRunning)
                    SizedBox(
                      height: 14,
                      width: 14,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        color: t.leaf,
                      ),
                    ),
                ],
              ),
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
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  SizedBox(
                    width: 100,
                    child: NubeField(
                      controller: _portController,
                      label: 'Port',
                      keyboardType: TextInputType.number,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: NubeButton(
                      label: scanState is LanScanRunning ? 'Stop' : 'Scan',
                      icon: scanState is LanScanRunning
                          ? LucideIcons.square
                          : LucideIcons.radar,
                      variant: NubeButtonVariant.secondary,
                      expand: true,
                      onPressed: scanState is LanScanRunning
                          ? () => ref
                              .read(lanScanControllerProvider.notifier)
                              .cancel()
                          : _startScan,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              _ScanResults(state: scanState, onApply: widget.onApply),
            ],
          ],
        ],
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
    final t = Theme.of(context).nube;

    switch (state) {
      case LanScanIdle():
        return Text(
          'Pick an interface and press Scan to discover rubix-agent instances on this /24 subnet.',
          style: TextStyle(color: t.muted, fontSize: 12.5, height: 1.4),
        );
      case LanScanFailed(:final message):
        return NubeAlert.error(message);
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
    final t = Theme.of(context).nube;
    final hits = progress.hits;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ClipRRect(
          borderRadius: BorderRadius.circular(999),
          child: LinearProgressIndicator(
            value: progress.total == 0 ? null : progress.percent,
            minHeight: 4,
            color: t.leaf,
            backgroundColor: t.surface2,
          ),
        ),
        const SizedBox(height: 6),
        Text(
          running
              ? '${progress.scanned}/${progress.total}'
                  '${progress.lastIp != null ? '  ·  ${progress.lastIp}' : ''}'
              : 'Done — ${hits.length} found',
          style: TextStyle(color: t.muted, fontSize: 12),
        ),
        const SizedBox(height: 8),
        if (hits.isEmpty && !running)
          Text(
            'No rubix-agent instances responded.',
            style: TextStyle(color: t.muted, fontSize: 12.5),
          )
        else
          ...hits.map(
            (h) => Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: NubeCard(
                onTap: () => onApply(h),
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                child: Row(
                  children: [
                    Icon(LucideIcons.server, size: 15, color: t.muted),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            h.baseUrl,
                            style: TextStyle(
                              color: t.text,
                              fontSize: 13,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          if (h.version != null)
                            Text(
                              'v${h.version}',
                              style:
                                  TextStyle(color: t.muted, fontSize: 11.5),
                            ),
                        ],
                      ),
                    ),
                    Icon(LucideIcons.chevronRight, size: 14, color: t.muted),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _WebUnsupportedNotice extends StatelessWidget {
  const _WebUnsupportedNotice();

  @override
  Widget build(BuildContext context) {
    return const NubeAlert.info(
      'LAN scan is not available in the browser. Run rubix from the desktop or mobile app to discover agents automatically, or enter the URL by hand below.',
    );
  }
}
