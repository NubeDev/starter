import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/network/interfaces/interface_models.dart';
import 'package:rubix_flutter/core/network/interfaces/interface_provider.dart';

/// Icon for each [InterfaceType].
IconData _iconFor(InterfaceType type) {
  switch (type) {
    case InterfaceType.ethernet:
      return Icons.settings_ethernet;
    case InterfaceType.wifi:
      return Icons.wifi;
    case InterfaceType.loopback:
      return Icons.loop;
    case InterfaceType.vpn:
      return Icons.vpn_lock;
    case InterfaceType.bridge:
      return Icons.device_hub;
    case InterfaceType.other:
      return Icons.device_unknown;
  }
}

/// A dropdown that lists available network interfaces (eth0, wlan0, …).
///
/// Uses [networkInterfacesProvider] to load interfaces and
/// [selectedNetworkInterfaceProvider] to track the selection.
///
/// ```dart
/// NetworkInterfaceSelector(
///   onChanged: (iface) => print(iface?.name),
/// )
/// ```
class NetworkInterfaceSelector extends ConsumerWidget {
  const NetworkInterfaceSelector({
    super.key,
    this.onChanged,
    this.label = 'Network Interface',
    this.showAddresses = true,
    this.includeLoopback = false,
  });

  /// Called when the user picks an interface. Receives `null` if unselected.
  final ValueChanged<NetworkInterfaceInfo?>? onChanged;

  /// Label shown above / inside the dropdown.
  final String label;

  /// Whether to show the IP address alongside the interface name.
  final bool showAddresses;

  /// Whether to include the loopback interface in the list.
  final bool includeLoopback;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ifacesAsync = ref.watch(networkInterfacesProvider);
    final selected = ref.watch(selectedNetworkInterfaceProvider);

    return ifacesAsync.when(
      loading: () => _buildDropdown(
        context,
        ref,
        items: const [],
        selected: null,
        loading: true,
      ),
      error: (e, _) => _ErrorTile(label: label, error: e),
      data: (ifaces) {
        final filtered = includeLoopback
            ? ifaces
            : ifaces
                .where((i) => i.type != InterfaceType.loopback)
                .toList();

        // Auto-select first interface if none selected yet.
        if (selected == null && filtered.isNotEmpty) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            ref
                .read(selectedNetworkInterfaceProvider.notifier)
                .select(filtered.first);
            onChanged?.call(filtered.first);
          });
        }

        return _buildDropdown(
          context,
          ref,
          items: filtered,
          selected: filtered.contains(selected) ? selected : null,
        );
      },
    );
  }

  Widget _buildDropdown(
    BuildContext context,
    WidgetRef ref, {
    required List<NetworkInterfaceInfo> items,
    required NetworkInterfaceInfo? selected,
    bool loading = false,
  }) {
    return DropdownButtonFormField<NetworkInterfaceInfo>(
      decoration: InputDecoration(
        labelText: label,
        prefixIcon: selected != null
            ? Icon(_iconFor(selected.type))
            : const Icon(Icons.device_unknown),
        border: const OutlineInputBorder(),
        suffixIcon: loading
            ? const Padding(
                padding: EdgeInsets.all(12),
                child: SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              )
            : null,
      ),
      // ignore: deprecated_member_use
      value: selected,
      hint: loading
          ? const Text('Loading interfaces…')
          : const Text('Select interface'),
      items: items
          .map(
            (iface) => DropdownMenuItem<NetworkInterfaceInfo>(
              value: iface,
              child: _InterfaceTile(
                iface: iface,
                showAddress: showAddresses,
              ),
            ),
          )
          .toList(),
      onChanged: loading
          ? null
          : (iface) {
              ref
                  .read(selectedNetworkInterfaceProvider.notifier)
                  .select(iface);
              onChanged?.call(iface);
            },
    );
  }
}

class _InterfaceTile extends StatelessWidget {
  const _InterfaceTile({required this.iface, required this.showAddress});

  final NetworkInterfaceInfo iface;
  final bool showAddress;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final addr = iface.primaryAddress;

    return Row(
      children: [
        Icon(_iconFor(iface.type), size: 18),
        const SizedBox(width: 8),
        Expanded(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(iface.name, style: theme.textTheme.bodyMedium),
              if (showAddress && addr != null)
                Text(
                  addr,
                  style: theme.textTheme.bodySmall
                      ?.copyWith(color: theme.colorScheme.outline),
                ),
            ],
          ),
        ),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          decoration: BoxDecoration(
            color: theme.colorScheme.secondaryContainer,
            borderRadius: BorderRadius.circular(4),
          ),
          child: Text(
            iface.type.label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.onSecondaryContainer,
            ),
          ),
        ),
      ],
    );
  }
}

class _ErrorTile extends StatelessWidget {
  const _ErrorTile({required this.label, required this.error});

  final String label;
  final Object error;

  @override
  Widget build(BuildContext context) {
    return InputDecorator(
      decoration: InputDecoration(
        labelText: label,
        border: const OutlineInputBorder(),
        prefixIcon: Icon(
          Icons.error_outline,
          color: Theme.of(context).colorScheme.error,
        ),
      ),
      child: Text(
        'Failed to load interfaces: $error',
        style: TextStyle(color: Theme.of(context).colorScheme.error),
        overflow: TextOverflow.ellipsis,
      ),
    );
  }
}
