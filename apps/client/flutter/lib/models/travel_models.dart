/// 目的地与景点模型。
///
/// 解析容错：字段缺失/类型不符时回落默认值，保证后端字段未就绪时页面不崩。
/// 名称按 13 语种列（name_en/name_zh/...）或 name_en+name_zh 两种形态取本地化值。
class Destination {
  const Destination({required this.id, required this.name, this.coverUrl = '', this.description = ''});

  final int id;
  final String name;
  final String coverUrl;
  final String description;

  static Destination fromJson(Map<String, dynamic> json, String lang) {
    final name = localizedName(json, lang) ?? 'ID ${json['id'] ?? json['region_id']}';
    return Destination(
      id: (json['id'] as num?)?.toInt() ?? (json['region_id'] as num?)?.toInt() ?? 0,
      name: name,
      coverUrl: (json['cover_url'] as String?) ?? '',
      description: decodeDescription(json['description'], lang),
    );
  }
}

class Attraction {
  const Attraction({
    required this.id,
    required this.destinationId,
    required this.name,
    this.description = '',
    this.priceCents = 0,
    this.openHours = '',
    this.rating = 0.0,
    this.coverUrl = '',
  });

  final int id;
  final int destinationId;
  final String name;
  final String description;
  final int priceCents;
  final String openHours;
  final double rating;
  final String coverUrl;

  static Attraction fromJson(Map<String, dynamic> json, String lang) {
    final name = localizedName(json, lang) ?? 'ID ${json['id']}';
    return Attraction(
      id: (json['id'] as num?)?.toInt() ?? 0,
      destinationId: (json['destination_id'] as num?)?.toInt() ?? 0,
      name: name,
      description: decodeDescription(json['description'], lang),
      priceCents: (json['price_cents'] as num?)?.toInt() ?? 0,
      openHours: (json['open_hours'] as String?) ?? '',
      rating: (json['rating_avg'] as num?)?.toDouble() ?? 0.0,
      coverUrl: (json['cover_url'] as String?) ?? '',
    );
  }
}

/// 优先当前语种列，其次 zh（多数用户），最后 en。
String? localizedName(Map<String, dynamic> json, String lang) {
  for (final key in [lang, 'zh', 'en']) {
    final v = json['name_$key'];
    if (v is String && v.isNotEmpty) return v;
  }
  return null;
}

/// description 可能为 JSON 对象 {en:.., zh:..}（DB 形态）或直接字符串。
String decodeDescription(dynamic raw, String lang) {
  if (raw is String) return raw.trim().isEmpty ? '' : raw;
  if (raw is Map) {
    for (final key in [lang, 'zh', 'en']) {
      final v = raw[key];
      if (v is String && v.isNotEmpty) return v;
    }
  }
  return '';
}

/// 分 → 元：12300 → ¥123，12350 → ¥123.5，12345 → ¥123.45
String formatYuan(int cents) {
  final s = (cents / 100).toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
  return '¥$s';
}
