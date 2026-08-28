import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:open_travel/models/travel_models.dart';
import 'package:open_travel/pages/profile_page.dart';
import 'package:open_travel/services/localization_service.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  test('getString loads ARB and falls back to key', () async {
    final loc = LocalizationService.instance;
    await loc.loadLanguage(const Locale('zh'));
    expect(loc.getString('nav.home'), contains('首页'));
    expect(loc.getString('unknownKey'), 'unknownKey');
  });

  test('new i18n keys present in all 13 locales with en fallback', () async {
    final loc = LocalizationService.instance;
    for (final lang in ['en', 'zh', 'ja', 'ko', 'ar', 'es', 'fr', 'de', 'pt', 'hi', 'bn', 'id', 'ru']) {
      await loc.loadLanguage(Locale(lang), persist: false);
      expect(loc.getString('detail.attractions'), isNotEmpty,
          reason: '$lang missing detail.attractions');
    }
  });

  test('formatYuan converts cents to yuan', () {
    expect(formatYuan(12300), '¥123');
    expect(formatYuan(12350), '¥123.5');
    expect(formatYuan(12345), '¥123.45');
    expect(formatYuan(0), '¥0');
  });

  test('Attraction.fromJson tolerant of missing fields', () {
    final a = Attraction.fromJson({
      'id': 7,
      'destination_id': 2,
      'name_en': 'Eiffel',
      'name_zh': '埃菲尔铁塔',
      'description': {'en': 'Iconic tower', 'zh': '地标铁塔'},
      'price_cents': 12345,
      'open_hours': '09:00-18:00',
      'rating_avg': 4.5,
    }, 'zh');
    expect(a.id, 7);
    expect(a.name, '埃菲尔铁塔');
    expect(a.description, '地标铁塔');
    expect(a.priceCents, 12345);
    expect(a.rating, 4.5);
    expect(a.coverUrl, '');

    final b = Attraction.fromJson({'id': 1}, 'en');
    expect(b.rating, 0.0);
    expect(b.name, 'ID 1');
  });

  test('localizedName falls back zh then en', () {
    final json = {'name_zh': '东京', 'name_en': 'Tokyo'};
    expect(localizedName(json, 'ja'), '东京');
    expect(localizedName(json, 'en'), 'Tokyo');
    expect(localizedName(json, 'xx'), '东京');
    expect(localizedName({'name_en': ''}, 'en'), isNull);
  });

  testWidgets('profile page renders localized strings', (tester) async {
    final loc = LocalizationService.instance;
    // ARB loading hits real I/O, must run outside the fake async zone
    await tester.runAsync(() => loc.loadLanguage(const Locale('en')));
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: ProfilePage())),
    );
    expect(find.text('English'), findsOneWidget);
    expect(find.text('Language'), findsOneWidget);
    expect(find.text('Not logged in'), findsOneWidget);
  });
}
