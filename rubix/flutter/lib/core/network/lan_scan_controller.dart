/// Riverpod controller for the LAN-scan panel on the Add Connection
/// screen. Wraps [scanLan] with start/cancel/clear plumbing and exposes
/// the latest [LanScanProgress] for the UI.
library;

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:rubix_flutter/core/network/lan_scanner.dart';

/// UI-facing scan state.
sealed class LanScanState {
  const LanScanState();
}

class LanScanIdle extends LanScanState {
  const LanScanIdle();
}

class LanScanRunning extends LanScanState {
  const LanScanRunning(this.progress);
  final LanScanProgress progress;
}

class LanScanDone extends LanScanState {
  const LanScanDone(this.progress);
  final LanScanProgress progress;
}

class LanScanFailed extends LanScanState {
  const LanScanFailed(this.message);
  final String message;
}

class LanScanController extends Notifier<LanScanState> {
  StreamSubscription<LanScanProgress>? _sub;

  @override
  LanScanState build() {
    ref.onDispose(() {
      _sub?.cancel();
    });
    return const LanScanIdle();
  }

  Future<void> start({required String localIp, int port = 8088}) async {
    await _sub?.cancel();
    if (deriveSubnet24(localIp) == null) {
      state = LanScanFailed('Not a scannable IPv4 address: "$localIp"');
      return;
    }
    LanScanProgress? last;
    final stream = scanLan(localIp, port: port);
    state = const LanScanRunning(
      LanScanProgress(scanned: 0, total: 254, hits: []),
    );
    _sub = stream.listen(
      (p) {
        last = p;
        state = LanScanRunning(p);
      },
      onError: (Object e) {
        state = LanScanFailed(e.toString());
      },
      onDone: () {
        state = LanScanDone(
          last ?? const LanScanProgress(scanned: 0, total: 0, hits: []),
        );
      },
    );
  }

  Future<void> cancel() async {
    await _sub?.cancel();
    _sub = null;
    state = const LanScanIdle();
  }

  void clear() {
    _sub?.cancel();
    _sub = null;
    state = const LanScanIdle();
  }
}

final lanScanControllerProvider =
    NotifierProvider<LanScanController, LanScanState>(LanScanController.new);
