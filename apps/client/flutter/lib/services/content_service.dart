import '../models/travel_models.dart';
import 'api_client.dart';
import 'localization_service.dart';

/// 内容接口：热门目的地 + 目的地景点。
class ContentService {
  ContentService._();

  static final ContentService instance = ContentService._();

  String get _lang => LocalizationService.instance.locale.languageCode;

  Future<List<Destination>> fetchDestinations({int regionId = 1}) async {
    final res = await ApiClient.instance.dio
        .get<Map<String, dynamic>>('/api/v1/booking/dates', queryParameters: {'region_id': regionId});
    final data = res.data?['data'];
    if (data is! List) return const [];
    return [
      for (final item in data)
        if (item is Map<String, dynamic>) Destination.fromJson(item, _lang),
    ];
  }

  Future<List<Attraction>> fetchAttractions({required int destinationId}) async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/v1/booking/attractions',
      queryParameters: {'destination_id': destinationId},
    );
    final data = res.data?['data'];
    if (data is! List) return const [];
    return [
      for (final item in data)
        if (item is Map<String, dynamic>) Attraction.fromJson(item, _lang),
    ];
  }
}
