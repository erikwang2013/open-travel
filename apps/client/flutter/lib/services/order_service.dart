import 'package:dio/dio.dart';

import '../models/travel_models.dart';
import 'api_client.dart';
import 'auth_service.dart';
import 'localization_service.dart';

/// 搜索 + 线路 + 订单接口封装。
class OrderService {
  OrderService._();

  static final OrderService instance = OrderService._();

  String get _lang => LocalizationService.instance.locale.languageCode;

  Options get _auth => AuthService.instance.authOptions;

  Future<SearchResults> search({
    String q = '',
    int? destinationId,
    int? priceMin,
    int? priceMax,
    int page = 1,
  }) async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/v1/search',
      queryParameters: {
        'q': q.isEmpty ? null : q,
        'destination_id': ?destinationId,
        'lang': _lang,
        'price_min': ?priceMin,
        'price_max': ?priceMax,
        'page': page,
      },
    );
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) {
      return SearchResults(total: 0, page: page, pageSize: 0, items: const []);
    }
    return SearchResults(
      total: (data['total'] as num?)?.toInt() ?? 0,
      page: (data['page'] as num?)?.toInt() ?? page,
      pageSize: (data['page_size'] as num?)?.toInt() ?? 0,
      items: [
        for (final item in (data['items'] as List? ?? const []))
          if (item is Map<String, dynamic>) SearchItem.fromJson(item, _lang),
      ],
    );
  }

  Future<List<Line>> fetchLines({int? destinationId}) async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/v1/lines',
      queryParameters: {'destination_id': ?destinationId, 'lang': _lang},
    );
    final data = res.data?['data'];
    if (data is! List) return const [];
    return [
      for (final item in data)
        if (item is Map<String, dynamic>) Line.fromJson(item, _lang),
    ];
  }

  Future<Line> fetchLine(int id) async {
    final res = await ApiClient.instance.dio
        .get<Map<String, dynamic>>('/api/v1/lines/$id', queryParameters: {'lang': _lang});
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) throw Exception('bad response');
    return Line.fromJson(data, _lang);
  }

  Future<List<LineDate>> fetchLineDates(int id) async {
    final res = await ApiClient.instance.dio
        .get<Map<String, dynamic>>('/api/v1/lines/$id/dates', queryParameters: {'lang': _lang});
    final data = res.data?['data'];
    if (data is! List) return const [];
    return [
      for (final item in data)
        if (item is Map<String, dynamic>) LineDate.fromJson(item, _lang),
    ];
  }

  /// order_type=1 线路：lineDateId 为班期 id/date；2 航班/3 酒店：可带 check_in/check_out。
  Future<Order> createOrder({
    required int productId,
    required Object lineDateId,
    required int quantity,
    int orderType = 1,
    String? checkIn,
    String? checkOut,
  }) async {
    final res = await ApiClient.instance.dio.post<Map<String, dynamic>>(
      '/api/v1/orders',
      data: {
        'order_type': orderType,
        'product_id': productId,
        if (orderType == 1) 'line_date_id': lineDateId,
        'quantity': quantity,
        'check_in': ?checkIn,
        'check_out': ?checkOut,
      },
      options: _auth,
    );
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) throw Exception('bad response');
    return Order.fromJson(data, _lang);
  }

  Future<List<Flight>> searchFlights({
    required String from,
    required String to,
    String? departDate,
    int? cabin,
  }) async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/v1/flights/search',
      queryParameters: {
        'from': from,
        'to': to,
        'depart_date': departDate,
        'cabin': cabin,
      },
    );
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) return const [];
    return [
      for (final item in (data['items'] as List? ?? const []))
        if (item is Map<String, dynamic>) Flight.fromJson(item),
    ];
  }

  Future<Flight> fetchFlight(int id) async {
    final res =
        await ApiClient.instance.dio.get<Map<String, dynamic>>('/api/v1/flights/$id');
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) throw Exception('bad response');
    return Flight.fromJson(data);
  }

  Future<List<Hotel>> searchHotels({required String city, int? star}) async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/v1/hotels/search',
      queryParameters: {'city': city, 'star': star},
    );
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) return const [];
    return [
      for (final item in (data['items'] as List? ?? const []))
        if (item is Map<String, dynamic>) Hotel.fromJson(item, _lang),
    ];
  }

  Future<Hotel> fetchHotel(int id) async {
    final res =
        await ApiClient.instance.dio.get<Map<String, dynamic>>('/api/v1/hotels/$id');
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) throw Exception('bad response');
    return Hotel.fromJson(data, _lang);
  }

  Future<List<PaymentChannel>> fetchPaymentChannels() async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/v1/payments/channels',
      queryParameters: {'lang': _lang},
    );
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) return const [];
    return [
      for (final item in (data['items'] as List? ?? const []))
        if (item is Map<String, dynamic>) PaymentChannel.fromJson(item, _lang),
    ];
  }

  Future<PaymentResult> createPayment({
    required int orderId,
    required String channelCode,
  }) async {
    final res = await ApiClient.instance.dio.post<Map<String, dynamic>>(
      '/api/v1/payments',
      data: {'order_id': orderId, 'channel_code': channelCode},
      options: _auth,
    );
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) throw Exception('bad response');
    return PaymentResult(
      txnNo: (data['txn_no'] as String?) ?? '',
      amountCents: (data['amount_cents'] as num?)?.toInt() ?? 0,
      checkoutUrl: (data['checkout_url'] as String?) ?? '',
    );
  }

  Future<OrderPage> fetchOrders({int page = 1, int pageSize = 20}) async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/v1/orders',
      queryParameters: {'page': page, 'page_size': pageSize},
      options: _auth,
    );
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) {
      return OrderPage(total: 0, page: page, pageSize: pageSize, items: const []);
    }
    return OrderPage(
      total: (data['total'] as num?)?.toInt() ?? 0,
      page: (data['page'] as num?)?.toInt() ?? page,
      pageSize: (data['page_size'] as num?)?.toInt() ?? pageSize,
      items: [
        for (final item in (data['items'] as List? ?? const []))
          if (item is Map<String, dynamic>) Order.fromJson(item, _lang),
      ],
    );
  }

  Future<Order> fetchOrder(int id) async {
    final res = await ApiClient.instance.dio
        .get<Map<String, dynamic>>('/api/v1/orders/$id', options: _auth);
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) throw Exception('bad response');
    return Order.fromJson(data, _lang);
  }

  Future<Order> cancelOrder(int id) async {
    final res = await ApiClient.instance.dio
        .post<Map<String, dynamic>>('/api/v1/orders/$id/cancel', options: _auth);
    final data = res.data?['data'];
    if (data is! Map<String, dynamic>) throw Exception('bad response');
    return Order.fromJson(data, _lang);
  }
}
