import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/identify/presentation/identity_card.dart';
import 'package:provision_app/features/place/domain/placement.dart';
import 'package:provision_app/features/place/presentation/place_step.dart';
import 'package:provision_app/features/provision/presentation/provisioned_reveal.dart';
import 'package:provision_app/features/provision/presentation/toggles_step.dart';
import 'package:provision_app/features/scan/domain/build_provision_input.dart';
import 'package:provision_app/features/scan/presentation/scanner.dart';
import 'package:provision_app/features/scan/presentation/type_picker.dart';
import 'package:provision_app/shared/widgets/chip.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/page_header.dart';
import 'package:provision_app/shared/widgets/primary_button.dart';
import 'package:provision_app/shared/widgets/toast.dart';

/// The phone flow controller: Scan → Identify → Place → Toggles → Confirm.
/// Each step is a screen; one `bc_provision` call lands the device and pops the
/// success reveal. [onPreview] jumps the shell to the Page preview tab. Ported
/// from the React `ScanFlow.tsx`.
class ScanFlow extends ConsumerStatefulWidget {
  const ScanFlow({required this.onPreview, super.key});

  final void Function(String pageId) onPreview;

  @override
  ConsumerState<ScanFlow> createState() => _ScanFlowState();
}

enum _Step { scan, identify, place, confirm, done }

const _stepLabel = <_Step, String>{
  _Step.scan: 'Step 1 · Scan',
  _Step.identify: 'Step 2 · Identify',
  _Step.place: 'Step 3 · Place',
  _Step.confirm: 'Step 4 · Confirm',
  _Step.done: 'Done',
};
const _stepTitle = <_Step, String>{
  _Step.scan: 'Add a device',
  _Step.identify: 'Identify',
  _Step.place: 'Where does it go?',
  _Step.confirm: 'Trending & alarming',
  _Step.done: 'Provisioned',
};

class _ScanFlowState extends ConsumerState<ScanFlow> {
  _Step _step = _Step.scan;
  bool _typeMode = false;
  String _barcode = '';
  ScannedIdentity? _identity;
  Placement _place = Placement.empty;
  final TextEditingController _name = TextEditingController();
  bool _trend = true;
  bool _alarm = true;
  bool _busy = false;
  ProvisionResult? _result;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _reset() {
    setState(() {
      _step = _Step.scan;
      _typeMode = false;
      _barcode = '';
      _identity = null;
      _place = Placement.empty;
      _name.clear();
      _trend = true;
      _alarm = true;
      _result = null;
    });
  }

  void _fault(Object e) => ref
      .read(toastProvider.notifier)
      .show(e.toString(), accent: RubixTokens.fault);

  Future<void> _onCode(String raw) async {
    setState(() {
      _barcode = raw;
      _busy = true;
    });
    try {
      final id = await ref.read(bcApiProvider).decode(raw);
      if (!mounted) return;
      setState(() {
        _identity = id;
        _step = _Step.identify;
      });
    } catch (e) {
      _fault(e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _confirm() async {
    setState(() => _busy = true);
    try {
      final name = _name.text.trim();
      final r = await ref.read(bcApiProvider).provision(
            buildProvisionInput(
              _barcode,
              _place,
              _trend,
              _alarm,
              name: name.isEmpty ? null : name,
            ),
          );
      if (!mounted) return;
      setState(() {
        _result = r;
        _step = _Step.done;
      });
    } catch (e) {
      _fault(e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _back() {
    setState(() {
      _step = switch (_step) {
        _Step.confirm => _Step.place,
        _Step.place => _Step.identify,
        _ => _Step.scan,
      };
    });
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final showBack = _step != _Step.scan && _step != _Step.done;

    return Stack(
      children: [
        SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(20, 80, 20, 128),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  if (showBack) ...[
                    _BackButton(onTap: _back),
                    const SizedBox(width: 8),
                  ],
                  Expanded(
                    child: PageHeader(
                      eyebrow: _stepLabel[_step]!,
                      title: _stepTitle[_step]!,
                    ),
                  ),
                ],
              ),
              AnimatedSwitcher(
                duration: const Duration(milliseconds: 200),
                transitionBuilder: (child, anim) => FadeTransition(
                  opacity: anim,
                  child: SlideTransition(
                    position: Tween(
                      begin: const Offset(0, 0.05),
                      end: Offset.zero,
                    ).animate(anim),
                    child: child,
                  ),
                ),
                child: KeyedSubtree(
                  key: ValueKey(_step),
                  child: _buildStep(look.accent),
                ),
              ),
            ],
          ),
        ),
        if (_step == _Step.done && _result != null)
          Positioned.fill(
            child: ProvisionedReveal(
              result: _result!,
              onPreview: _result!.pageId.isNotEmpty
                  ? () {
                      final pid = _result!.pageId;
                      _reset();
                      widget.onPreview(pid);
                    }
                  : null,
              onAddAnother: _reset,
            ),
          ),
      ],
    );
  }

  Widget _buildStep(Color accent) {
    switch (_step) {
      case _Step.scan:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                GlassChip(
                  label: 'Scan barcode',
                  active: !_typeMode,
                  onTap: () => setState(() => _typeMode = false),
                ),
                const SizedBox(width: 8),
                GlassChip(
                  label: 'Pick a type',
                  active: _typeMode,
                  onTap: () => setState(() => _typeMode = true),
                ),
              ],
            ),
            const SizedBox(height: 16),
            if (_busy) ...[
              const _DecodingHint(),
              const SizedBox(height: 16),
            ],
            if (_typeMode)
              TypePicker(onSynthesized: _onCode)
            else
              Scanner(onCode: _onCode),
            const SizedBox(height: 24),
            const _ScanHint(),
          ],
        );
      case _Step.identify:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            IdentityCard(identity: _identity!),
            const SizedBox(height: 20),
            PrimaryButton(
              label: 'Place this device',
              accent: accent,
              onPressed: () => setState(() => _step = _Step.place),
            ),
          ],
        );
      case _Step.place:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            PlaceStep(
              value: _place,
              onChanged: (p) => setState(() => _place = p),
            ),
            const SizedBox(height: 20),
            PrimaryButton(
              label: 'Continue',
              accent: accent,
              onPressed: _place.ready
                  ? () => setState(() => _step = _Step.confirm)
                  : null,
            ),
            if (!_place.ready)
              const _InfoHint(
                'Pick a site to continue — a page is optional.',
              )
            else if (!_place.hasPage)
              const _InfoHint(
                'No page chosen — it’ll be commissioned as pending. Place it '
                'on a page anytime from Devices.',
              ),
          ],
        );
      case _Step.confirm:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Field(
              label: 'Device name',
              child: GlassTextField(
                controller: _name,
                hint: _identity?.id.isNotEmpty ?? false
                    ? _identity!.id
                    : 'e.g. L3 North Droplet',
              ),
            ),
            const SizedBox(height: 20),
            const SectionLabel('Behavior'),
            TogglesStep(
              trend: _trend,
              alarm: _alarm,
              onTrend: (v) => setState(() => _trend = v),
              onAlarm: (v) => setState(() => _alarm = v),
            ),
            if (!_place.hasPage)
              const _InfoHint(
                'Commissioned as pending — place it on a page anytime from '
                'Devices.',
              ),
            const SizedBox(height: 20),
            PrimaryButton(
              label: _busy ? 'Provisioning…' : 'Add device',
              accent: accent,
              loading: _busy,
              onPressed: _busy ? null : _confirm,
            ),
          ],
        );
      case _Step.done:
        return const SizedBox.shrink();
    }
  }
}

class _BackButton extends StatelessWidget {
  const _BackButton({required this.onTap});
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: Semantics(
          button: true,
          label: 'Back',
          child: Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: Colors.white.withValues(alpha: 0.06),
              shape: BoxShape.circle,
            ),
            child: const Icon(
              LucideIcons.arrowLeft,
              size: 20,
              color: RubixTokens.inkVariant,
            ),
          ),
        ),
      ),
    );
  }
}

class _DecodingHint extends StatelessWidget {
  const _DecodingHint();

  @override
  Widget build(BuildContext context) {
    return const Row(
      children: [
        SizedBox(
          width: 16,
          height: 16,
          child: CircularProgressIndicator(
            strokeWidth: 2,
            color: RubixTokens.inkMuted,
          ),
        ),
        SizedBox(width: 8),
        Text(
          'Decoding…',
          style: TextStyle(color: RubixTokens.inkMuted, fontSize: 14),
        ),
      ],
    );
  }
}

class _ScanHint extends StatelessWidget {
  const _ScanHint();

  @override
  Widget build(BuildContext context) {
    return const Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Icon(LucideIcons.scanLine, size: 14, color: RubixTokens.inkMuted),
        SizedBox(width: 8),
        Flexible(
          child: Text(
            'QR or Code128 · or paste a rubix://add URL',
            style: TextStyle(color: RubixTokens.inkMuted, fontSize: 12),
          ),
        ),
      ],
    );
  }
}

class _InfoHint extends StatelessWidget {
  const _InfoHint(this.text);
  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(LucideIcons.info, size: 14, color: RubixTokens.inkMuted),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              text,
              style: const TextStyle(
                color: RubixTokens.inkMuted,
                fontSize: 12,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
