import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:shared_preferences/shared_preferences.dart';

class LocalizationService extends ChangeNotifier {
  LocalizationService._();

  static final LocalizationService instance = LocalizationService._();

  static const _langKey = 'app_lang';

  static const List<Locale> supportedLocales = [
    Locale('en'),
    Locale('zh'),
    Locale('ja'),
    Locale('ko'),
    Locale('ru'),
    Locale('de'),
    Locale('fr'),
    Locale('es'),
    Locale('pt'),
    Locale('hi'),
    Locale('ar'),
    Locale('bn'),
    Locale('id'),
  ];

  static const Set<String> _rtlLanguages = {'ar'};

  Locale _locale = const Locale('en');
  Map<String, dynamic>? _strings;
  Map<String, dynamic>? _en;

  Locale get locale => _locale;
  bool get isRtl => _rtlLanguages.contains(_locale.languageCode);

  /// Persisted choice first, else system locale when supported, else en.
  Future<void> init() async {
    final prefs = await SharedPreferences.getInstance();
    final saved = prefs.getString(_langKey);
    Locale locale;
    if (saved != null && supportedLocales.any((l) => l.languageCode == saved)) {
      locale = Locale(saved);
    } else {
      final system = WidgetsBinding.instance.platformDispatcher.locale;
      locale = supportedLocales.any((l) => l.languageCode == system.languageCode)
          ? Locale(system.languageCode)
          : const Locale('en');
    }
    await loadLanguage(locale, persist: false);
  }

  Future<void> loadLanguage(Locale locale, {bool persist = true}) async {
    _strings = await _load('assets/i18n/${locale.languageCode}.arb');
    if (locale.languageCode != 'en') {
      _en ??= await _load('assets/i18n/en.arb');
    }
    _locale = locale;
    if (persist) {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_langKey, locale.languageCode);
    }
    notifyListeners();
  }

  /// Current language first, then en, then the key itself as last resort.
  String getString(String key) {
    final value = _strings?[key] ?? _en?[key];
    return value is String ? value : key;
  }

  Future<Map<String, dynamic>> _load(String path) async =>
      jsonDecode(await rootBundle.loadString(path)) as Map<String, dynamic>;
}
