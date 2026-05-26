import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/network/interfaces/interface_models.dart';

/// Loads all non-loopback network interfaces from the OS.
/// Pass [includeLoopback] = true to also include `lo`.
Future<List<NetworkInterfaceInfo>> loadNetworkInterfaces({
  bool includeLoopback = false,
}) async {
  final raw = await NetworkInterface.list(
    includeLoopback: includeLoopback,
  );

  return raw.map((iface) {
    final addresses = iface.addresses.map((a) => a.address).toList();
    final type = InterfaceType.fromName(iface.name);
    return NetworkInterfaceInfo(
      name: iface.name,
      addresses: addresses,
      type: type,
    );
  }).toList();
}

/// Async provider that lists all available network interfaces.
final networkInterfacesProvider =
    FutureProvider<List<NetworkInterfaceInfo>>((ref) async {
  return loadNetworkInterfaces();
});

/// Tracks the currently selected [NetworkInterfaceInfo].
/// Defaults to the first non-loopback interface once the list loads.
class NetworkInterfaceSelectionNotifier
    extends Notifier<NetworkInterfaceInfo?> {
  @override
  NetworkInterfaceInfo? build() => null;

  // ignore: use_setters_to_change_properties
  void select(NetworkInterfaceInfo? iface) => state = iface;
}

final selectedNetworkInterfaceProvider =
    NotifierProvider<NetworkInterfaceSelectionNotifier, NetworkInterfaceInfo?>(
  NetworkInterfaceSelectionNotifier.new,
);
