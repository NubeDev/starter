import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/network/network_providers.dart';
import 'package:provision_app/core/network/ping_result.dart';
import 'package:provision_app/core/network/tenant.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/auth/data/auth_controller.dart';
import 'package:provision_app/shared/widgets/bottom_sheet.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/glass_card.dart';
import 'package:provision_app/shared/widgets/primary_button.dart';

/// The default agent base URL — overridable at build time with
/// `--dart-define=AGENT_URL=http://192.168.1.50:8088`. Falls back to localhost
/// (correct for desktop, and for a device using `adb reverse`).
const _defaultAgentUrl =
    String.fromEnvironment('AGENT_URL', defaultValue: 'http://127.0.0.1:8088');

/// Gate screen: agent base URL + email/password. Shown until authed. Ported
/// from the React `Connect`. Pre-fills the host this device last connected to
/// (survives logout), falling back to the compiled-in default on first run.
class ConnectScreen extends ConsumerStatefulWidget {
  const ConnectScreen({super.key});

  @override
  ConsumerState<ConnectScreen> createState() => _ConnectScreenState();
}

class _ConnectScreenState extends ConsumerState<ConnectScreen> {
  late final TextEditingController _baseUrl;
  final _email = TextEditingController(text: 'op@example.com');
  final _password = TextEditingController(text: 'rubix-dev-passwd');

  bool _pinging = false;
  PingResult? _pingResult;

  @override
  void initState() {
    super.initState();
    final saved = ref.read(transportProvider).savedBaseUrl;
    _baseUrl = TextEditingController(
      text: saved.isNotEmpty ? saved : _defaultAgentUrl,
    );
  }

  @override
  void dispose() {
    _baseUrl.dispose();
    _email.dispose();
    _password.dispose();
    super.dispose();
  }

  Future<void> _doPing() async {
    if (_pinging) return;
    setState(() {
      _pinging = true;
      _pingResult = null;
    });
    final result = await ref.read(authControllerProvider.notifier).ping(
          _baseUrl.text.trim(),
        );
    if (mounted) {
      setState(() {
        _pinging = false;
        _pingResult = result;
      });
    }
  }

  Future<void> _submit({String? tenantId}) async {
    final auth = ref.read(authControllerProvider);
    if (auth.busy) return;
    try {
      await ref.read(authControllerProvider.notifier).login(
            _baseUrl.text.trim(),
            _email.text.trim(),
            _password.text,
            tenantId: tenantId,
          );
    } on TenantRequiredException catch (e) {
      // Multi-org account — let the operator pick which org to sign into, then
      // retry login bound to that tenant.
      if (!mounted) return;
      final picked = await _pickTenant(e.memberships);
      if (picked != null) await _submit(tenantId: picked);
    } catch (_) {
      // Other failures surface via authState.error in the form.
    }
  }

  /// Org picker shown when the agent reports the account belongs to multiple
  /// tenants. Returns the chosen `tenant_id`, `*` for "All orgs" (admins), or
  /// null if dismissed.
  Future<String?> _pickTenant(List<TenantMembership> memberships) {
    final isAdmin = memberships.any((m) => m.role == 'admin');
    return showGlassBottomSheet<String>(
      context: context,
      title: 'Choose an org',
      builder: (sheetCtx) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (isAdmin) ...[
            _TenantTile(
              label: 'All orgs',
              sub: 'See every org (admin)',
              icon: LucideIcons.globe,
              onTap: () => Navigator.of(sheetCtx).pop(superAdminTenant),
            ),
            const SizedBox(height: 8),
          ],
          for (final m in memberships) ...[
            _TenantTile(
              label: m.tenantId,
              sub: m.role,
              icon: LucideIcons.building2,
              onTap: () => Navigator.of(sheetCtx).pop(m.tenantId),
            ),
            const SizedBox(height: 8),
          ],
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final auth = ref.watch(authControllerProvider);

    // Transparent Scaffold so the global obsidian base + MoodBackdrop show
    // through, while still providing the Material/DefaultTextStyle ancestor the
    // Text widgets need (without it they render in the yellow debug style).
    return Scaffold(
      backgroundColor: Colors.transparent,
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.fromLTRB(
              RubixTokens.margin,
              56,
              RubixTokens.margin,
              64,
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
              // ── brand ──────────────────────────────────────────────────
              Container(
                width: 64,
                height: 64,
                decoration: BoxDecoration(
                  color: look.themeAccent,
                  borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
                  boxShadow: GlassShadow.glow(look.themeAccent),
                ),
                child: const Icon(
                  LucideIcons.scanLine,
                  size: 32,
                  color: RubixTokens.primaryOn,
                ),
              ),
              const SizedBox(height: 16),
              const Text('Rubix Provision', style: RubixText.headlineMobile),
              const SizedBox(height: 4),
              const Text(
                'Scan a device. Place it. Done.',
                style: TextStyle(color: RubixTokens.inkVariant, fontSize: 14),
              ),
              const SizedBox(height: 32),

              // ── form ───────────────────────────────────────────────────
              GlassSurface(
                borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
                boxShadow: GlassShadow.glass,
                padding: const EdgeInsets.all(20),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      children: [
                        const Icon(LucideIcons.plug,
                            size: 16, color: RubixTokens.inkMuted),
                        const SizedBox(width: 8),
                        Text('BROWSER · DIRECT REST', style: RubixText.label),
                      ],
                    ),
                    const SizedBox(height: 16),
                    Field(
                      label: 'Agent base URL',
                      child: GlassTextField(
                        controller: _baseUrl,
                        keyboardType: TextInputType.url,
                        hint: 'http://127.0.0.1:8088',
                        onChanged: (_) => setState(() => _pingResult = null),
                      ),
                    ),
                    const SizedBox(height: 16),
                    _PingButton(
                      pinging: _pinging,
                      onTap: _doPing,
                    ),
                    if (_pingResult != null) ...[
                      const SizedBox(height: 12),
                      _PingVerdict(result: _pingResult!),
                    ],
                    const SizedBox(height: 16),
                    Field(
                      label: 'Email',
                      child: GlassTextField(
                        controller: _email,
                        keyboardType: TextInputType.emailAddress,
                        hint: 'op@example.com',
                      ),
                    ),
                    const SizedBox(height: 16),
                    Field(
                      label: 'Password',
                      child: GlassTextField(
                        controller: _password,
                        obscureText: true,
                        hint: '••••••••',
                        onSubmitted: _submit,
                      ),
                    ),
                    if (auth.error != null) ...[
                      const SizedBox(height: 16),
                      Text(
                        auth.error!,
                        style: const TextStyle(
                          color: RubixTokens.fault,
                          fontSize: 14,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ],
                    const SizedBox(height: 16),
                    PrimaryButton(
                      label: auth.busy ? 'Connecting…' : 'Connect',
                      accent: look.themeAccent,
                      loading: auth.busy,
                      onPressed: _submit,
                    ),
                  ],
                ),
              ),
            ],
          ),
          ),
        ),
      ),
    );
  }
}

class _PingButton extends StatelessWidget {
  const _PingButton({required this.pinging, required this.onTap});
  final bool pinging;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: pinging ? null : onTap,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: Opacity(
          opacity: pinging ? 0.6 : 1,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
              border: Border.all(color: Colors.white.withValues(alpha: 0.15)),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                if (pinging)
                  const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(
                        strokeWidth: 2, color: RubixTokens.ink),
                  )
                else
                  const Icon(LucideIcons.wifi,
                      size: 16, color: RubixTokens.ink),
                const SizedBox(width: 8),
                Text(
                  pinging ? 'Pinging…' : 'Ping agent',
                  style: const TextStyle(
                    color: RubixTokens.ink,
                    fontSize: 14,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _PingVerdict extends StatelessWidget {
  const _PingVerdict({required this.result});
  final PingResult result;

  @override
  Widget build(BuildContext context) {
    final color = result.ok ? RubixTokens.online : RubixTokens.fault;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(
          result.ok ? LucideIcons.checkCircle2 : LucideIcons.xCircle,
          size: 16,
          color: color,
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            result.message,
            style: TextStyle(
              color: color,
              fontSize: 14,
              fontWeight: FontWeight.w500,
            ),
          ),
        ),
      ],
    );
  }
}

/// One row in the org picker sheet — a tappable glass card.
class _TenantTile extends StatelessWidget {
  const _TenantTile({
    required this.label,
    required this.sub,
    required this.icon,
    required this.onTap,
  });

  final String label;
  final String sub;
  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GlassCard(
      onTap: onTap,
      padding: const EdgeInsets.all(14),
      child: Row(
        children: [
          Icon(icon, size: 20, color: RubixTokens.primary),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  label,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: RubixTokens.ink,
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  sub,
                  style: const TextStyle(
                    color: RubixTokens.inkMuted,
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
          const Icon(LucideIcons.chevronRight,
              size: 18, color: RubixTokens.inkMuted),
        ],
      ),
    );
  }
}
