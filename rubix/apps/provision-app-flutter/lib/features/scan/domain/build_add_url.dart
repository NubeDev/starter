/// Build the canonical `rubix://add?…` payload that a sticker QR encodes. This
/// is the single source of truth for the on-sticker grammar — the TypePicker
/// (synthesize-from-type) and the Templates QR generator both call it so a
/// hand-made sticker scans back through `bc_decode` identically. Ported from
/// the React `buildAddUrl.ts`.
///
/// [model] is the device *type* (must match a template); the address slot is
/// `eui` for lora, `addr` for bacnet, and `ip` for rubix/REST.
String buildAddUrl({
  required String id,
  required String model,
  required String network,
  String? address,
}) {
  final params = <String, String>{
    'v': '1',
    'id': id,
    'model': model,
    'network': network,
  };
  final addr = address?.trim() ?? '';
  if (addr.isNotEmpty) {
    final slot = network == 'bacnet'
        ? 'addr'
        : network == 'lora'
            ? 'eui'
            : 'ip';
    params[slot] = addr;
  }
  final query = Uri(queryParameters: params).query;
  return 'rubix://add?$query';
}

/// Human label for the address field, per the template's network.
String addressLabel(String network) {
  switch (network) {
    case 'lora':
      return 'DevEUI';
    case 'bacnet':
      return 'BACnet address';
    default:
      return 'IP address';
  }
}
