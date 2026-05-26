/// `GET /api/v1/ui/table` query + response types.
///
/// Mirrors `crates/starter-sdui-routes/src/query.rs` and `table.rs`.
///
/// Pure Dart — no Flutter imports.
library;

class TableQuery {
  const TableQuery({
    required this.sourceId,
    this.page = 1,
    this.size = 50,
    this.sort,
    this.filter,
  });

  final String sourceId;
  final int page;
  final int size;

  /// Comma-separated sort spec, e.g. `"name,-created_at"`.
  final String? sort;

  /// RSQL filter expression.
  final String? filter;

  Map<String, String> toQueryParams() => {
        'source_id': sourceId,
        'page': '$page',
        'size': '$size',
        if (sort != null) 'sort': sort!,
        if (filter != null) 'filter': filter!,
      };
}

class TableResponse {
  const TableResponse({
    required this.rows,
    required this.total,
    required this.page,
    required this.size,
  });

  final List<Map<String, Object?>> rows;
  final int total;
  final int page;
  final int size;

  factory TableResponse.fromJson(Map<String, Object?> map) => TableResponse(
        rows: ((map['rows'] as List?) ?? const [])
            .map((e) => (e as Map).cast<String, Object?>())
            .toList(),
        total: (map['total'] as num?)?.toInt() ?? 0,
        page: (map['page'] as num?)?.toInt() ?? 1,
        size: (map['size'] as num?)?.toInt() ?? 0,
      );
}
