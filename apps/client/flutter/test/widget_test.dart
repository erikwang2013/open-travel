import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:open_travel/pages/profile_page.dart';
import 'package:open_travel/services/localization_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('getString loads ARB and falls back to key', () async {
    final loc = LocalizationService.instance;
    await loc.loadLanguage(const Locale('zh'));
    expect(loc.getString('nav.home'), contains('首页'));
    expect(loc.getString('unknownKey'), 'unknownKey');
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
  });
}
