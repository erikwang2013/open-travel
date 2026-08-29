import 'dart:convert';

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

/// 搜索接口混合结果：destination / attraction 两类。
class SearchItem {
  const SearchItem({
    required this.id,
    required this.type,
    required this.name,
    this.priceCents = 0,
    this.coverUrl = '',
    this.description = '',
  });

  final int id;
  final String type;
  final String name;
  final int priceCents;
  final String coverUrl;
  final String description;

  bool get isDestination => type == 'destination';

  static SearchItem fromJson(Map<String, dynamic> json, String lang) {
    final name = localizedName(json, lang) ?? (json['name'] as String?) ?? 'ID ${json['id']}';
    return SearchItem(
      id: (json['id'] as num?)?.toInt() ?? 0,
      type: (json['type'] as String?) ?? 'attraction',
      name: name,
      priceCents: (json['price_cents'] as num?)?.toInt() ?? 0,
      coverUrl: (json['cover_url'] as String?) ?? '',
      description: decodeDescription(json['description'], lang),
    );
  }
}

class SearchResults {
  const SearchResults({required this.total, required this.page, required this.pageSize, required this.items});

  final int total;
  final int page;
  final int pageSize;
  final List<SearchItem> items;
}

class ItineraryDay {
  const ItineraryDay({required this.day, this.title = '', this.description = ''});

  final int day;
  final String title;
  final String description;

  static ItineraryDay fromJson(Map<String, dynamic> json, String lang) => ItineraryDay(
        day: (json['day'] as num?)?.toInt() ?? 0,
        title: localizedName(json, lang) ?? (json['title'] as String?) ?? '',
        description: decodeDescription(json['description'], lang),
      );
}

/// 旅游线路。title 可能走 name_* 列或纯 title 字段，两种都兼容。
class Line {
  const Line({
    required this.id,
    required this.title,
    this.destinationId = 0,
    this.days = 0,
    this.priceCents = 0,
    this.maxPax = 0,
    this.coverUrl = '',
    this.itinerary = const [],
  });

  final int id;
  final String title;
  final int destinationId;
  final int days;
  final int priceCents;
  final int maxPax;
  final String coverUrl;
  final List<ItineraryDay> itinerary;

  static Line fromJson(Map<String, dynamic> json, String lang) {
    final title = localizedName(json, lang) ?? (json['title'] as String?) ?? 'ID ${json['id']}';
    return Line(
      id: (json['id'] as num?)?.toInt() ?? 0,
      title: title,
      destinationId: (json['destination_id'] as num?)?.toInt() ?? 0,
      days: (json['days'] as num?)?.toInt() ?? 0,
      priceCents: (json['price_cents'] as num?)?.toInt() ?? 0,
      maxPax: (json['max_pax'] as num?)?.toInt() ?? 0,
      coverUrl: (json['cover_url'] as String?) ?? '',
      itinerary: [
        for (final i in (json['itinerary'] as List? ?? const []))
          if (i is Map<String, dynamic>) ItineraryDay.fromJson(i, lang),
      ],
    );
  }
}

class LineDate {
  const LineDate({required this.date, this.id = 0, this.priceCents = 0, this.seatsLeft = 0, this.soldOut = false});

  final String date;

  /// 后端未返回 id 时为 0，下单时回落用 date 字符串。
  final int id;
  final int priceCents;
  final int seatsLeft;
  final bool soldOut;

  bool get available => !soldOut && seatsLeft > 0;

  static LineDate fromJson(Map<String, dynamic> json, String lang) => LineDate(
        date: (json['date'] as String?) ?? '',
        id: (json['id'] as num?)?.toInt() ?? 0,
        priceCents: (json['price_cents'] as num?)?.toInt() ?? 0,
        seatsLeft: (json['seats_left'] as num?)?.toInt() ?? 0,
        soldOut: (json['sold_out'] as bool?) ?? false,
      );
}

class Order {
  const Order({
    required this.id,
    this.orderType = 0,
    this.productId = 0,
    this.amountCents = 0,
    this.status = 0,
    this.createdAt = '',
    this.productSnapshot,
  });

  final int id;
  final int orderType;
  final int productId;
  final int amountCents;
  final int status;
  final String createdAt;

  /// 产品快照：JSON 对象或字符串，展示时容错解析。
  final dynamic productSnapshot;

  bool get isPending => status == 0;

  static Order fromJson(Map<String, dynamic> json, String lang) => Order(
        id: (json['id'] as num?)?.toInt() ?? 0,
        orderType: (json['order_type'] as num?)?.toInt() ?? 0,
        productId: (json['product_id'] as num?)?.toInt() ?? 0,
        amountCents: (json['amount_cents'] as num?)?.toInt() ?? 0,
        status: (json['status'] as num?)?.toInt() ?? 0,
        createdAt: (json['created_at'] as String?) ?? '',
        productSnapshot: json['product_snapshot'],
      );
}

class OrderPage {
  const OrderPage({required this.total, required this.page, required this.pageSize, required this.items});

  final int total;
  final int page;
  final int pageSize;
  final List<Order> items;
}

/// 订单状态文案 key：0待支付/1已支付/2已确认/3已完成/4已取消。
String orderStatusKey(int status) {
  switch (status) {
    case 1:
      return 'booking.status.paid';
    case 2:
      return 'order.status.confirmed';
    case 3:
      return 'order.status.completed';
    case 4:
      return 'booking.status.cancelled';
    default:
      return 'booking.status.pending';
  }
}

/// 航班。cabin: 0经济/1商务/2头等。
class Flight {
  const Flight({
    required this.id,
    this.airline = '',
    this.flightNo = '',
    this.fromCode = '',
    this.toCode = '',
    this.departAt = '',
    this.arriveAt = '',
    this.cabin = 0,
    this.priceCents = 0,
    this.seatsLeft = 0,
  });

  final int id;
  final String airline;
  final String flightNo;
  final String fromCode;
  final String toCode;
  final String departAt;
  final String arriveAt;
  final int cabin;
  final int priceCents;
  final int seatsLeft;

  bool get soldOut => seatsLeft <= 0;

  static Flight fromJson(Map<String, dynamic> json) => Flight(
        id: (json['id'] as num?)?.toInt() ?? 0,
        airline: (json['airline'] as String?) ?? '',
        flightNo: (json['flight_no'] as String?) ?? '',
        fromCode: (json['from_code'] as String?) ?? '',
        toCode: (json['to_code'] as String?) ?? '',
        departAt: (json['depart_at'] as String?) ?? '',
        arriveAt: (json['arrive_at'] as String?) ?? '',
        cabin: (json['cabin'] as num?)?.toInt() ?? 0,
        priceCents: (json['price_cents'] as num?)?.toInt() ?? 0,
        seatsLeft: (json['seats_left'] as num?)?.toInt() ?? 0,
      );
}

/// 舱位 i18n key：0经济/1商务/2头等。
String flightCabinKey(int cabin) => switch (cabin) {
      1 => 'flight.cabin.business',
      2 => 'flight.cabin.first',
      _ => 'flight.cabin.economy',
    };

/// 酒店房型。名称按 room_type_en/zh/ja 多语列取。
class HotelRoom {
  const HotelRoom({
    required this.id,
    this.name = '',
    this.priceCents = 0,
    this.breakfast = false,
    this.inventory = 0,
  });

  final int id;
  final String name;
  final int priceCents;
  final bool breakfast;
  final int inventory;

  bool get available => inventory > 0;

  static HotelRoom fromJson(Map<String, dynamic> json, String lang) {
    String? name;
    for (final key in [lang, 'zh', 'en']) {
      final v = json['room_type_$key'];
      if (v is String && v.isNotEmpty) {
        name = v;
        break;
      }
    }
    return HotelRoom(
      id: (json['id'] as num?)?.toInt() ?? 0,
      name: name ?? 'ID ${json['id']}',
      priceCents: (json['price_cents'] as num?)?.toInt() ?? 0,
      breakfast: (json['breakfast'] as bool?) ?? false,
      inventory: (json['inventory'] as num?)?.toInt() ?? 0,
    );
  }
}

/// 酒店。name 按 name_* 多语列取。
class Hotel {
  const Hotel({
    required this.id,
    this.name = '',
    this.cityCode = '',
    this.star = 0,
    this.coverUrl = '',
    this.rooms = const [],
  });

  final int id;
  final String name;
  final String cityCode;
  final int star;
  final String coverUrl;
  final List<HotelRoom> rooms;

  static Hotel fromJson(Map<String, dynamic> json, String lang) => Hotel(
        id: (json['id'] as num?)?.toInt() ?? 0,
        name: localizedName(json, lang) ?? 'ID ${json['id']}',
        cityCode: (json['city_code'] as String?) ?? '',
        star: (json['star'] as num?)?.toInt() ?? 0,
        coverUrl: (json['cover_url'] as String?) ?? '',
        rooms: [
          for (final r in (json['rooms'] as List? ?? const []))
            if (r is Map<String, dynamic>) HotelRoom.fromJson(r, lang),
        ],
      );
}

/// 支付渠道。name 为多语 JSON（已按 lang 路由）或字符串。
class PaymentChannel {
  const PaymentChannel({
    required this.channelCode,
    this.name = '',
    this.type = '',
    this.enabled = false,
    this.priority = 0,
  });

  final String channelCode;
  final String name;
  final String type;
  final bool enabled;
  final int priority;

  static PaymentChannel fromJson(Map<String, dynamic> json, String lang) {
    var name = json['name'];
    if (name is Map) {
      name = localizedName(Map<String, dynamic>.from(name), lang) ?? '';
    }
    return PaymentChannel(
      channelCode: (json['channel_code'] as String?) ?? '',
      name: name is String ? name : '',
      type: (json['type'] as String?) ?? '',
      enabled: (json['enabled'] as bool?) ?? false,
      priority: (json['priority'] as num?)?.toInt() ?? 0,
    );
  }
}

/// 发起支付返回：流水号 + 金额 + 沙箱收银台地址。
class PaymentResult {
  const PaymentResult({this.txnNo = '', this.amountCents = 0, this.checkoutUrl = ''});

  final String txnNo;
  final int amountCents;
  final String checkoutUrl;
}

/// 订单产品快照取标题：Map 形态（title/name/name_*）或字符串（含 JSON 字符串）容错解析。
String snapshotTitle(dynamic snapshot, String lang) {
  if (snapshot is Map) {
    for (final key in ['title', 'name', 'name_$lang', 'name_zh', 'name_en']) {
      final v = snapshot[key];
      if (v is String && v.isNotEmpty) return v;
    }
    return '';
  }
  if (snapshot is String && snapshot.isNotEmpty) {
    try {
      final decoded = jsonDecode(snapshot);
      if (decoded is Map) return snapshotTitle(decoded, lang);
    } on FormatException {
      // fall through: 非 JSON 字符串直接展示
    }
    return snapshot;
  }
  return '';
}
