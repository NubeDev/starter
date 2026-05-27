import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';
import 'package:rubix_sdui/rubix_sdui.dart';

/// Renders a server-driven UI page resolved from
/// `POST /api/v1/ui/resolve`. See
/// packages/rubix_sdui/docs/PROOF.md.
class SduiPageScreen extends ConsumerStatefulWidget {
  const SduiPageScreen({required this.pageRef, super.key});

  final String pageRef;

  @override
  ConsumerState<SduiPageScreen> createState() => _SduiPageScreenState();
}

class _SduiPageScreenState extends ConsumerState<SduiPageScreen> {
  SduiNotifier? _notifier;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_notifier != null) return;

    final dio = ref.read(dioProvider);
    if (dio == null) return;

    final notifier = SduiNotifier(service: SduiService(dio: dio));
    _notifier = notifier;
    notifier.load(pageRef: widget.pageRef);
  }

  @override
  void dispose() {
    _notifier?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final notifier = _notifier;
    return Scaffold(
      appBar: AppBar(title: Text(widget.pageRef)),
      body: notifier == null
          ? const _NoConnectionView()
          : SduiProvider(
              notifier: notifier,
              child: const SduiRenderer(),
            ),
    );
  }
}

class _NoConnectionView extends StatelessWidget {
  const _NoConnectionView();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Padding(
        padding: EdgeInsets.all(24),
        child: Text(
          'No active connection — add one in Connections first.',
          textAlign: TextAlign.center,
        ),
      ),
    );
  }
}
